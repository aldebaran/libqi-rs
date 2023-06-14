use crate::{
    message::Id,
    request::{Call, Cancel, Capabilities, Event, Post, Request},
};
use bytes::Bytes;
use futures::{future, ready, FutureExt, Sink, SinkExt, Stream, StreamExt};
use std::{
    collections::HashMap,
    fmt::Debug,
    task::{Context, Poll},
};
use tokio::{
    pin, select,
    sync::{mpsc, oneshot},
};
use tokio_util::sync::PollSender;
use tracing::debug;

#[derive(Debug)]
pub(crate) struct Client {
    dispatch_sender: PollSender<DispatchRequest>,
}

impl Client {
    pub(crate) fn new<Si, St>(
        responses_stream: St,
        requests_sink: Si,
    ) -> (
        Self,
        impl std::future::Future<Output = Result<(), Si::Error>>,
    )
    where
        Si: Sink<Request>,
        Si::Error: std::error::Error,
        St: Stream<Item = (Id, Result<Bytes, CallError>)>,
    {
        const DISPATCH_CHANNEL_SIZE: usize = 1;
        let (dispatch_sender, dispatch_receiver) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
        let dispatch_sender = PollSender::new(dispatch_sender);
        let dispatch = dispatch(dispatch_receiver, requests_sink, responses_stream);
        (Self { dispatch_sender }, dispatch)
    }

    pub(crate) fn poll_dispatch_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.dispatch_sender
            .poll_reserve(cx)
            .map_err(|_err| DispatchError::Terminated.into())
    }
}

impl tower::Service<Request> for Client {
    type Response = Option<Bytes>;
    type Error = Error;
    type Future = ResponseFuture;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_dispatch_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        match request {
            Request::Call(call) => self.call(call).into(),
            Request::Post(post) => self.call(post).into(),
            Request::Event(event) => self.call(event).into(),
            Request::Cancel(cancel) => self.call(cancel).into(),
            Request::Capabilities(capabilities) => self.call(capabilities).into(),
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub(crate) enum ResponseFuture {
    Call(CallResponseFuture),
    NoResponse,
    Error(Option<Error>),
}

impl std::future::Future for ResponseFuture {
    type Output = Result<Option<Bytes>, Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            Self::Call(call) => call.poll_unpin(cx).map_ok(Some),
            Self::NoResponse => Poll::Ready(Ok(None)),
            Self::Error(err) => match err.take() {
                Some(err) => Poll::Ready(Err(err)),
                None => Poll::Pending,
            },
        }
    }
}

impl From<CallResponseFuture> for ResponseFuture {
    fn from(future: CallResponseFuture) -> Self {
        match future {
            CallResponseFuture::WaitForResponse(..) => Self::Call(future),
            CallResponseFuture::Error(err) => Self::Error(err),
        }
    }
}

impl From<NoResponseFuture> for ResponseFuture {
    fn from(future: NoResponseFuture) -> Self {
        match future.into_inner() {
            Ok(()) => ResponseFuture::NoResponse,
            Err(err) => ResponseFuture::Error(Some(err)),
        }
    }
}

impl tower::Service<Call> for Client {
    type Response = Bytes;
    type Error = Error;
    type Future = CallResponseFuture;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.dispatch_sender
            .poll_reserve(cx)
            .map_err(|_err| DispatchError::Terminated.into())
    }

    fn call(&mut self, request: Call) -> Self::Future {
        let (response_sender, response_receiver) = oneshot::channel();
        match self.dispatch_sender.send_item(DispatchRequest::Call {
            request,
            response_sender,
        }) {
            Ok(()) => CallResponseFuture::WaitForResponse(response_receiver),
            Err(_send_err) => CallResponseFuture::Error(Some(DispatchError::Terminated.into())),
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub(crate) enum CallResponseFuture {
    WaitForResponse(oneshot::Receiver<Result<Bytes, CallError>>),
    Error(Option<Error>),
}

impl std::future::Future for CallResponseFuture {
    type Output = Result<Bytes, Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            CallResponseFuture::WaitForResponse(response_receiver) => {
                let res = match ready!(response_receiver.poll_unpin(cx)) {
                    Ok(Ok(reply)) => Ok(reply),
                    Ok(Err(req_err)) => Err(req_err.into()),
                    Err(_recv_err) => Err(DispatchError::RequestCanceled.into()),
                };
                Poll::Ready(res)
            }
            CallResponseFuture::Error(err) => match err.take() {
                Some(err) => Poll::Ready(Err(err)),
                None => Poll::Pending,
            },
        }
    }
}

pub(crate) type NoResponseFuture = future::Ready<Result<(), Error>>;

macro_rules! impl_service_for_no_response_requests {
    ($($req:ty),+) => {
        $(
            impl tower::Service<$req> for Client {
                type Response = ();
                type Error = Error;
                type Future = NoResponseFuture;

                fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                    self.dispatch_sender
                        .poll_reserve(cx)
                        .map_err(|_err| DispatchError::Terminated.into())
                }

                fn call(&mut self, request: $req) -> Self::Future {
                    let res = self
                        .dispatch_sender
                        .send_item(DispatchRequest::Other(request.into()))
                        .map_err(|_send_err| DispatchError::Terminated.into());
                    future::ready(res)
                }
            }
        )+
    };
}

impl_service_for_no_response_requests! {
    Post, Event, Cancel, Capabilities
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("dispatch error")]
    Dispatch(#[from] DispatchError),

    #[error(transparent)]
    Call(#[from] CallError),
}

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("the call request has been canceled")]
    Canceled,

    #[error("the call request ended with an error: {0}")]
    Error(String),
}

async fn dispatch<St, Si>(
    mut request_receiver: mpsc::Receiver<DispatchRequest>,
    requests_sink: Si,
    responses_stream: St,
) -> Result<(), Si::Error>
where
    Si: Sink<Request>,
    Si::Error: std::error::Error,
    St: Stream<Item = (Id, Result<Bytes, CallError>)>,
{
    let mut ongoing_call_requests = HashMap::new();
    let requests_sink = requests_sink;
    let responses_stream = responses_stream.fuse();
    pin!(responses_stream, requests_sink);

    loop {
        select! {
            Some(request) = request_receiver.recv() => {
                let request = match request {
                    DispatchRequest::Call {
                        request,
                        response_sender,
                    } => {
                        let id = request.id;
                        debug!(%id, "registering a call request waiting for a response from the server");
                        ongoing_call_requests.insert(id, response_sender);
                        request.into()
                    }
                    DispatchRequest::Other(request) => request,
                };
                requests_sink.send(request).await?;
            }
            Some((id, response)) = responses_stream.next() => {
                debug!(response = ?response, "received a call response from the server");
                if let Some(response_sender) = ongoing_call_requests.remove(&id) {
                    if let Err(response) = response_sender.send(response) {
                        debug!(response = ?response, "the client for a call request response has dropped, discarding response");
                    }
                }
            }
            else => {
                debug!("client dispatch is finished");
                break Ok(());
            }
        }

        // Cleanup ongoing call requests for which the client has dropped the channel.
        ongoing_call_requests.retain(|_id, response_sender| !response_sender.is_closed())
    }
}

#[derive(Debug)]
enum DispatchRequest {
    Call {
        request: Call,
        response_sender: oneshot::Sender<Result<Bytes, CallError>>,
    },
    Other(Request),
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("the dispatch task to remote is terminated")]
    Terminated,

    #[error("the dispatch task to remote has canceled the request")]
    RequestCanceled,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Subject;
    use assert_matches::assert_matches;
    use futures::future::{poll_immediate, BoxFuture};
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSendError;
    use tower::{Service, ServiceExt};

    struct TestClient {
        requests_rx: mpsc::Receiver<Request>,
        responses_tx: mpsc::Sender<(Id, Result<Bytes, CallError>)>,
        client: Client,
        dispatch: BoxFuture<'static, Result<(), PollSendError<Request>>>,
    }

    impl TestClient {
        fn new() -> Self {
            let (requests_tx, requests_rx) = mpsc::channel(1);
            let (responses_tx, responses_rx) = mpsc::channel(1);
            let requests_sink = PollSender::new(requests_tx);
            let responses_stream = ReceiverStream::new(responses_rx);
            let (client, dispatch) = Client::new(responses_stream, requests_sink);
            Self {
                requests_rx,
                responses_tx,
                client,
                dispatch: dispatch.boxed(),
            }
        }
    }

    #[tokio::test]
    async fn test_client_drop_dispatch_task_causes_terminated_error() {
        let mut test = TestClient::new();

        drop(test.dispatch);

        let res = ServiceExt::<Call>::ready(&mut test.client).await;
        assert_matches!(res, Err(Error::Dispatch(DispatchError::Terminated)));

        // Dropping the dispatch between the `ready` and the `call` doesn't make the `call` fail
        // as the `ready` will reserve a slot to send to the dispatch, so the send always succeeds
        // if `ready` succeeds, even if the dispatch is dropped in between.
        //
        // drop(dispatch);
        //
        // let call = service.call(Call {
        //     id: Id(1),
        //     subject: Subject::default(),
        //     payload: Bytes::from_static(&[1, 2, 3, 4]),
        // });
        //
        // assert_matches!(
        //     poll_immediate(call).await,
        //     Some(Err(Error::Dispatch(DispatchError::Terminated)))
        // );
    }

    #[tokio::test]
    async fn test_client_drop_dispatch_task_after_call_causes_canceled_error() {
        let mut test = TestClient::new();

        let call = ServiceExt::<Call>::ready(&mut test.client)
            .await
            .unwrap()
            .call(Call {
                id: Id(1),
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            });
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The dispatch is now waiting for the response. Drop the task.
        // It must cancel pending requests.
        drop(test.dispatch);
        assert_matches!(
            poll_immediate(call).await,
            Some(Err(Error::Dispatch(DispatchError::RequestCanceled)))
        );
    }

    #[tokio::test]
    async fn test_client_post() {
        let mut test = TestClient::new();

        let post = ServiceExt::<Post>::ready(&mut test.client)
            .await
            .unwrap()
            .call(Post {
                id: Id(1),
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            });

        pin!(post);

        assert_matches!(poll_immediate(&mut post).await, Some(Ok(())));
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(Request::Post(Post { id: Id(1), subject, payload, }))) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );
    }

    #[tokio::test]
    async fn test_client_call() {
        let mut test = TestClient::new();

        let call = ServiceExt::<Call>::ready(&mut test.client)
            .await
            .unwrap()
            .call(Call {
                id: Id(1),
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            });
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(Request::Call(Call { id: Id(1), subject, payload, }))) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        test.responses_tx
            .send((Id(1), Ok(Bytes::from_static(&[5, 6, 7, 8]))))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(poll_immediate(&mut call).await, Some(Ok(payload)) => {
            assert_eq!(payload, Bytes::from_static(&[5, 6, 7, 8]));
        });
    }

    #[tokio::test]
    async fn test_client_call_error() {
        let mut test = TestClient::new();

        let call = ServiceExt::<Call>::ready(&mut test.client)
            .await
            .unwrap()
            .call(Call {
                id: Id(1),
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            });
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(Request::Call(Call { id: Id(1), subject, payload, }))) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        test.responses_tx
            .send((Id(1), Err(CallError::Error("some error".to_owned()))))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(
            poll_immediate(&mut call).await,
            Some(Err(Error::Call(CallError::Error(err)))) => {
                assert_eq!(err, "some error");
            }
        );
    }

    #[tokio::test]
    async fn test_client_call_canceled() {
        let mut test = TestClient::new();

        let call = ServiceExt::<Call>::ready(&mut test.client)
            .await
            .unwrap()
            .call(Call {
                id: Id(1),
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            });
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(Request::Call(Call { id: Id(1), subject, payload, }))) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        test.responses_tx
            .send((Id(1), Err(CallError::Canceled)))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(
            poll_immediate(&mut call).await,
            Some(Err(Error::Call(CallError::Canceled)))
        );
    }

    #[tokio::test]
    async fn test_client_call_ignores_responses_of_other_id() {
        let mut test = TestClient::new();

        let call = ServiceExt::<Call>::ready(&mut test.client)
            .await
            .unwrap()
            .call(Call {
                id: Id(1),
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            });
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(Request::Call(Call { id: Id(1), subject, payload, }))) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        // Send a response of request id = 2.
        test.responses_tx
            .send((Id(2), Ok(Bytes::from_static(&[5, 6, 7, 8]))))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        // Send a response of request id = 1.
        test.responses_tx
            .send((Id(1), Ok(Bytes::from_static(&[9, 10, 11, 12]))))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(poll_immediate(&mut call).await, Some(Ok(payload)) => {
            assert_eq!(payload, Bytes::from_static(&[9, 10, 11, 12]));
        });
    }

    #[tokio::test]
    async fn test_client_sink_error_stops_dispatch_task() {
        let mut test = TestClient::new();

        // Drop the sink receiver, this will cause errors from the sender.
        drop(test.requests_rx);

        let call = ServiceExt::<Call>::ready(&mut test.client)
            .await
            .unwrap()
            .call(Call {
                id: Id(1),
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            });

        // The call cannot finish until dispatch is run.
        assert_matches!(poll_immediate(call).await, None);
        assert_matches!(poll_immediate(test.dispatch).await, Some(Err(_)));
    }
}
