use crate::request::{Request, Response};
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Sink, SinkExt, Stream, StreamExt,
};
use tokio::{pin, select};
use tower::{Service, ServiceExt};
use tracing::{debug, debug_span, instrument, Instrument};

#[instrument(level = "debug", skip_all, err)]
pub(crate) async fn serve<St, Si, Svc>(
    requests_stream: St,
    responses_sink: Si,
    service: Svc,
) -> Result<(), Error>
where
    St: Stream<Item = Request>,
    Si: Sink<Response>,
    Si::Error: Into<Box<dyn std::error::Error>>,
    Svc: tower::Service<Request, Response = Response>,
    Svc::Error: Into<Box<dyn std::error::Error>>,
{
    let mut requests_stream_terminated = false;
    let responses_sink = responses_sink.sink_map_err(|err| Error::Sink(err.into()));
    let mut service = service.map_err(|err| Error::Service(err.into()));
    let mut responses_futures = FuturesUnordered::new();
    pin!(requests_stream, responses_sink);

    loop {
        select! {
            request = requests_stream.next(), if !requests_stream_terminated => {
                match request {
                    Some(request) => {
                        debug!(?request, "received a new request, calling service");
                        let response_future = service.ready().await?.call(request).instrument(debug_span!("service_call"));
                        responses_futures.push(response_future);
                    }
                    None => {
                        debug!("request stream is terminated");
                        requests_stream_terminated = true;
                    }
                }
            },
            Some(response) = responses_futures.next(), if !responses_futures.is_terminated() => {
                let response = response?;
                debug!(?response, "received response of service call");
                responses_sink.send(response).await?;
            },
            else => {
                debug!("server is finished");
                break Ok(())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("output sink error")]
    Sink(#[source] Box<dyn std::error::Error>),

    #[error("service error")]
    Service(#[source] Box<dyn std::error::Error>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{message, request::RequestId};
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
        request_barriers: HashMap<RequestId, Arc<Barrier>>,
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
                Request::Call { id, subject, .. } => {
                    let barrier = self.request_barriers[&id].clone();
                    async move {
                        barrier.wait().await;
                        Ok(Response::reply(id, subject, Bytes::new()))
                    }
                    .boxed()
                }
                _ => {
                    let id = req.id();
                    let barrier = self.request_barriers[&id].clone();
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
                (RequestId::from(1), barrier_1.clone()),
                (RequestId::from(2), barrier_2.clone()),
                (RequestId::from(3), barrier_3.clone()),
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
                id: RequestId::from(1),
                subject,
                payload: Bytes::new(),
            })
            .await?;
        requests_tx
            .send(Request::Call {
                id: RequestId::from(2),
                subject,
                payload: Bytes::new(),
            })
            .await?;
        requests_tx
            .send(Request::Call {
                id: RequestId::from(3),
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
        assert_matches!(responses_rx.try_recv(), Ok(response) => response == Response::reply(RequestId::from(3), subject, Bytes::new()));

        // Unblock request no.1, its response is received.
        assert_matches!(poll_immediate(barrier_1.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(response) => response == Response::reply(RequestId::from(1), subject, Bytes::new()));

        // Unblock request no.2, its response is received.
        assert_matches!(poll_immediate(barrier_2.wait()).await, Some(_));
        assert_matches!(poll_immediate(&mut serve).await, None);
        assert_matches!(responses_rx.try_recv(), Ok(response) => response == Response::reply(RequestId::from(2), subject, Bytes::new()));

        // Terminate the server by closing the messages stream.
        drop(requests_tx);
        assert_matches!(poll_immediate(&mut serve).await, Some(Ok(())));

        Ok(())
    }
}
