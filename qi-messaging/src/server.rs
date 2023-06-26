use crate::messaging::{
    CallTermination, CallWithId, Message, NotificationWithId, Reply, RequestId, RequestWithId,
    Service, Subject, ToRequestId, ToSubject,
};
use futures::{stream::FuturesUnordered, FutureExt, Sink, SinkExt, Stream, StreamExt};
use tokio::{pin, select};
use tracing::{debug, debug_span, Instrument};

pub(crate) async fn serve<St, Si, Svc>(
    requests_stream: St,
    responses_sink: Si,
    mut service: Svc,
) -> Result<(), Si::Error>
where
    St: Stream<Item = RequestWithId>,
    Si: Sink<Response<Svc::Error>>,
    Svc: Service<CallWithId, NotificationWithId>,
    Svc::Error: std::fmt::Debug,
{
    let requests_stream = requests_stream.fuse();
    let mut responses_futures = FuturesUnordered::new();
    pin!(requests_stream, responses_sink);

    loop {
        select! {
            Some(request) = requests_stream.next() => {
                let (id, subject) = (request.to_request_id(), request.to_subject());
                debug!(?request, "received a new request, calling service");
                let response_future = service.request(request.transpose_id()).instrument(debug_span!("service_call"));
                responses_futures.push(response_future.map(move |response| (id, subject, response)));
            },
            Some((id, subject, response)) = responses_futures.next() => {
                debug!(%id, %subject, ?response, "received response of service call");
                if let Some(response) = response.transpose() {
                    responses_sink.send(Response { id, subject, response }).await?;
                }
            },
            else => {
                debug!("server is finished");
                break Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Response<E> {
    id: RequestId,
    subject: Subject,
    response: Result<Reply, CallTermination<E>>,
}

impl<E> TryFrom<Response<E>> for Message
where
    E: ToString,
{
    type Error = crate::format::Error;

    fn try_from(response: Response<E>) -> Result<Self, Self::Error> {
        match response.response {
            Ok(reply) => Ok(Message::reply(response.id, response.subject)
                .set_payload(reply.payload)
                .build()),
            Err(CallTermination::Canceled) => {
                Ok(Message::canceled(response.id, response.subject).build())
            }
            Err(CallTermination::Error(err)) => {
                Ok(Message::error(response.id, response.subject, &err.to_string())?.build())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        message,
        messaging::{Call, Request},
        service,
    };
    use assert_matches::assert_matches;
    use bytes::Bytes;
    use futures::{
        future::{poll_immediate, BoxFuture},
        FutureExt,
    };
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::{
        mpsc::{self, error::TryRecvError},
        Barrier,
    };
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSender;

    #[derive(Debug)]
    struct Service {
        request_barriers: HashMap<RequestId, Arc<Barrier>>,
    }

    impl<N> service::Service<CallWithId, N> for Service {
        type Error = String;
        type CallFuture = BoxFuture<'static, Result<Reply, CallTermination<Self::Error>>>;
        type NotifyFuture = BoxFuture<'static, Result<(), Self::Error>>;

        fn call(&mut self, call: CallWithId) -> Self::CallFuture {
            let id = call.to_request_id();
            let barrier = self.request_barriers.get(&id).cloned();
            async move {
                if let Some(barrier) = barrier {
                    barrier.wait().await;
                }
                Ok(Reply::with_value(&id).map_err(|err| err.to_string())?)
            }
            .boxed()
        }

        fn notify(&mut self, _notif: N) -> Self::NotifyFuture {
            unimplemented!()
        }
    }

    /// Tests that the service calls are executed concurrently as soon as a request is received
    /// without waiting for previous requests to be finished.
    #[tokio::test]
    async fn test_server_service_futures_execute_concurrently() {
        let (requests_tx, requests_rx) = mpsc::channel(4);
        let (responses_tx, mut responses_rx) = mpsc::channel(4);
        let barrier_1 = Arc::new(Barrier::new(2));
        let barrier_2 = Arc::new(Barrier::new(2));
        let barrier_3 = Arc::new(Barrier::new(2));
        let service = Service {
            request_barriers: [
                (RequestId::from(1), Arc::clone(&barrier_1)),
                (RequestId::from(2), Arc::clone(&barrier_2)),
                (RequestId::from(3), Arc::clone(&barrier_3)),
            ]
            .into_iter()
            .collect(),
        };

        let requests_stream = ReceiverStream::new(requests_rx);
        let responses_sink = PollSender::new(responses_tx);
        let serve = serve(requests_stream, responses_sink, service);
        pin!(serve);

        // Send 3 call requests.
        let subject = message::Subject::new(
            message::Service::new(1),
            message::Object::new(2),
            message::Action::new(3),
        );
        requests_tx
            .send(RequestWithId {
                id: RequestId::from(1),
                inner: Request::Call(Call {
                    subject,
                    payload: Bytes::new(),
                }),
            })
            .await
            .unwrap();
        requests_tx
            .send(RequestWithId {
                id: RequestId::from(2),
                inner: Request::Call(Call {
                    subject,
                    payload: Bytes::new(),
                }),
            })
            .await
            .unwrap();
        requests_tx
            .send(RequestWithId {
                id: RequestId::from(3),
                inner: Request::Call(Call {
                    subject,
                    payload: Bytes::new(),
                }),
            })
            .await
            .unwrap();

        // Poll the server once to process the requests.
        // Responses are not received yet, the service is awaiting.
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Err(TryRecvError::Empty));

        // Unblock request no.3, its response is received.
        assert_matches!(poll_immediate(barrier_3.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(Response{ id: RequestId(3), response: Ok(Reply { payload }), .. }) => {
            assert_eq!(payload, [3, 0, 0, 0].as_slice())
        });

        // Unblock request no.1, its response is received.
        assert_matches!(poll_immediate(barrier_1.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(Response{ id: RequestId(1), response: Ok(Reply { payload }), .. }) => {
            assert_eq!(payload, [1, 0, 0, 0].as_slice())
        });

        // Unblock request no.2, its response is received.
        assert_matches!(poll_immediate(barrier_2.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(Response{ id: RequestId(2), response: Ok(Reply { payload }), .. }) => {
            assert_eq!(payload, [2, 0, 0, 0].as_slice())
        });

        // Terminate the server by closing the messages stream.
        drop(requests_tx);
        assert_matches!(poll_immediate(&mut serve).await, Some(Ok(())));
    }

    #[tokio::test]
    async fn test_server_sink_error_stops_task() {
        let (requests_tx, requests_rx) = mpsc::channel(1);
        let (responses_tx, responses_rx) = mpsc::channel(1);
        let service = Service {
            request_barriers: HashMap::new(),
        };
        let requests_stream = ReceiverStream::new(requests_rx);
        let responses_sink = PollSender::new(responses_tx);

        let serve = serve(requests_stream, responses_sink, service);
        pin!(serve);

        // Drop the sink receiver, this will cause errors from the sender.
        drop(responses_rx);

        // Send a call request, causing the server to try to put a response in the sink.
        requests_tx
            .send(RequestWithId {
                id: RequestId::from(1),
                inner: Request::Call(Call {
                    subject: Subject::default(),
                    payload: Bytes::new(),
                }),
            })
            .await
            .unwrap();

        assert_matches!(poll_immediate(&mut serve).await, Some(Err(_err)));
    }
}
