use crate::{
    message::{Id, Message, Subject},
    request::{IsCanceledError, Request, TryIntoFailureMessage},
};
use bytes::Bytes;
use futures::{future::err, stream::FuturesUnordered, FutureExt, Sink, SinkExt, Stream, StreamExt};
use tokio::{pin, select};
use tower::ServiceExt;
use tracing::{debug, debug_span, Instrument};

pub(crate) async fn serve<St, Si, Svc>(
    requests_stream: St,
    responses_sink: Si,
    mut service: Svc,
) -> Result<(), Si::Error>
where
    St: Stream<Item = Request>,
    Si: Sink<Response<Svc::Response, Svc::Error>>,
    Svc: tower::Service<Request>,
    Svc::Response: std::fmt::Debug,
    Svc::Error: std::fmt::Debug,
{
    let requests_stream = requests_stream.fuse();
    let mut responses_futures = FuturesUnordered::new();
    pin!(requests_stream, responses_sink);

    loop {
        select! {
            Some(request) = requests_stream.next() => {
                let (id, subject) = (request.id(), request.subject());
                debug!(?request, "received a new request, calling service");
                let response_future = match service.ready().await {
                    Ok(service) => service.call(request).left_future(),
                    Err(error) => err(error).right_future(),
                }.instrument(debug_span!("service_call"));
                responses_futures.push(response_future.map(move |response| (id, subject, response)));
            },
            Some((id, subject, response)) = responses_futures.next() => {
                debug!(%id, %subject, ?response, "received response of service call");
                responses_sink.send(Response { id, subject, response }).await?;
            },
            else => {
                debug!("server is finished");
                break Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Response<R, E> {
    id: Id,
    subject: Subject,
    response: Result<R, E>,
}

impl<R, E> TryFrom<Response<R, E>> for Option<Message>
where
    R: Into<Option<Bytes>>,
    E: IsCanceledError + ToString,
{
    type Error = crate::format::Error;

    fn try_from(response: Response<R, E>) -> Result<Self, Self::Error> {
        match response.response {
            Ok(reply) => Ok(reply.into().map(|payload| {
                Message::reply(response.id, response.subject)
                    .set_payload(payload)
                    .build()
            })),
            Err(err) => err
                .try_into_failure_message(response.id, response.subject)
                .map(Some),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{message, request::Call};
    use assert_matches::assert_matches;
    use bytes::Bytes;
    use futures::{
        future::{poll_immediate, BoxFuture},
        FutureExt,
    };
    use std::{
        collections::HashMap,
        sync::Arc,
        task::{Context, Poll},
    };
    use tokio::sync::{
        mpsc::{self, error::TryRecvError},
        Barrier,
    };
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSender;

    #[derive(Debug)]
    struct Service {
        request_barriers: HashMap<Id, Arc<Barrier>>,
    }

    impl tower::Service<Request> for Service {
        type Response = Id;
        type Error = String;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Request) -> Self::Future {
            let id = req.id();
            let barrier = self.request_barriers.get(&id).cloned();
            async move {
                if let Some(barrier) = barrier {
                    barrier.wait().await;
                }
                Ok(id)
            }
            .boxed()
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
                (Id::from(1), Arc::clone(&barrier_1)),
                (Id::from(2), Arc::clone(&barrier_2)),
                (Id::from(3), Arc::clone(&barrier_3)),
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
            .send(Request::Call(Call {
                id: Id::from(1),
                subject,
                payload: Bytes::new(),
            }))
            .await
            .unwrap();
        requests_tx
            .send(Request::Call(Call {
                id: Id::from(2),
                subject,
                payload: Bytes::new(),
            }))
            .await
            .unwrap();
        requests_tx
            .send(Request::Call(Call {
                id: Id::from(3),
                subject,
                payload: Bytes::new(),
            }))
            .await
            .unwrap();

        // Poll the server once to process the requests.
        // Responses are not received yet, the service is awaiting.
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Err(TryRecvError::Empty));

        // Unblock request no.3, its response is received.
        assert_matches!(poll_immediate(barrier_3.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(response) => {
            assert_matches!(response, Response{ id: Id(3), response: Ok(Id(3)), .. });
        });

        // Unblock request no.1, its response is received.
        assert_matches!(poll_immediate(barrier_1.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(response) => {
            assert_matches!(response, Response{ id: Id(1), response: Ok(Id(1)), .. });
        });

        // Unblock request no.2, its response is received.
        assert_matches!(poll_immediate(barrier_2.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(response) => {
            assert_matches!(response, Response{ id: Id(2), response: Ok(Id(2)), .. });
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
            .send(Request::Call(Call {
                id: Id::from(1),
                subject: message::Subject::default(),
                payload: Bytes::new(),
            }))
            .await
            .unwrap();

        assert_matches!(poll_immediate(&mut serve).await, Some(Err(_err)));
    }
}
