use crate::{
    message::{Id, Subject},
    request::{Request, Response},
};
use futures::{
    future::err,
    stream::{FusedStream, FuturesUnordered},
    FutureExt, Sink, SinkExt, Stream, StreamExt,
};
use tokio::{pin, select};
use tower::ServiceExt;
use tracing::{debug, debug_span, instrument, Instrument};

#[instrument(level = "debug", skip_all)]
pub(crate) async fn serve<St, Si, Svc>(
    requests_stream: St,
    responses_sink: Si,
    mut service: Svc,
) -> Result<(), Si::Error>
where
    St: Stream<Item = Request>,
    Si: Sink<(Id, Subject, Response)>,
    Svc: tower::Service<Request, Response = Response>,
    Svc::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
{
    let mut requests_stream_terminated = false;
    let mut responses_futures = FuturesUnordered::new();
    pin!(requests_stream, responses_sink);

    loop {
        select! {
            request = requests_stream.next(), if !requests_stream_terminated => {
                match request {
                    Some(request) => {
                        let (id, subject) = (request.id(), request.subject());
                        debug!(?request, "received a new request, calling service");
                        let response_future = match service.ready().await {
                            Ok(service) => service.call(request).left_future(),
                            Err(error) => err(error).right_future(),
                        }.instrument(debug_span!("service_call"));
                        responses_futures.push(response_future.map(move |response| (id, subject, response)));
                    }
                    None => {
                        debug!("request stream is terminated");
                        requests_stream_terminated = true;
                    }
                }
            },
            Some((id, subject, response)) = responses_futures.next(), if !responses_futures.is_terminated() => {
                let response = response.unwrap_or_else(Response::error);
                debug!(?response, "received response of service call");
                responses_sink.send((id, subject, response)).await?;
            },
            else => {
                debug!("server is finished");
                break Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message;
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
        type Response = Response;
        type Error = String;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Request) -> Self::Future {
            match req {
                Request::Call { id, .. } => {
                    let barrier = Arc::clone(&self.request_barriers[&id]);
                    async move {
                        barrier.wait().await;
                        Ok(Response::reply(&()).unwrap())
                    }
                    .boxed()
                }
                _ => {
                    let id = req.id();
                    let barrier = Arc::clone(&self.request_barriers[&id]);
                    async move {
                        barrier.wait().await;
                        Ok(Response::none())
                    }
                    .boxed()
                }
            }
        }
    }

    /// Tests that the service calls are executed concurrently as soon as a request is received, without waiting for previous requests to be finished.
    #[tokio::test]
    async fn test_service_futures_execute_concurrently() -> Result<(), Box<dyn std::error::Error>> {
        let (requests_tx, requests_rx) = mpsc::channel(8);
        let (responses_tx, mut responses_rx) = mpsc::channel(8);
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
            .send(Request::Call {
                id: Id::from(1),
                subject,
                payload: Bytes::new(),
            })
            .await?;
        requests_tx
            .send(Request::Call {
                id: Id::from(2),
                subject,
                payload: Bytes::new(),
            })
            .await?;
        requests_tx
            .send(Request::Call {
                id: Id::from(3),
                subject,
                payload: Bytes::new(),
            })
            .await?;

        // Poll the server once to process the requests.
        // Responses are not received yet, the service is awaiting.
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Err(TryRecvError::Empty));

        // Unblock request no.3, its response is received.
        assert_matches!(poll_immediate(barrier_3.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(response) => {
            assert_matches!(response, (Id(3), _, Response(Some(_))));
        });

        // Unblock request no.1, its response is received.
        assert_matches!(poll_immediate(barrier_1.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(response) => {
            assert_matches!(response, (Id(1), _, Response(Some(_))));
        });

        // Unblock request no.2, its response is received.
        assert_matches!(poll_immediate(barrier_2.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(response) => {
            assert_matches!(response, (Id(2), _, Response(Some(_))));
        });

        // Terminate the server by closing the messages stream.
        drop(requests_tx);
        assert_matches!(poll_immediate(&mut serve).await, Some(Ok(())));

        Ok(())
    }
}
