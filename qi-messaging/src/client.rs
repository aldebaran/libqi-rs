use crate::{
    messaging::{
        self, Call, CallResult, Cancel, Notification, Reply, RequestId, RequestWithId, Service,
        Subject, ToRequestId,
    },
    GetSubject,
};
use futures::{
    future::{BoxFuture, FusedFuture},
    ready, FutureExt, Sink, SinkExt, Stream, StreamExt,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicU32, Arc},
    task::{Context, Poll},
};
use tokio::{
    pin, select,
    sync::{mpsc, oneshot},
    task,
};
use tokio_util::sync::PollSender;
use tracing::trace;

pub(crate) fn setup<Si, St>(
    responses_stream: St,
    requests_sink: Si,
) -> (Client, impl Future<Output = Result<(), Si::Error>>)
where
    Si: Sink<RequestWithId>,
    Si::Error: std::error::Error,
    St: Stream<Item = (RequestId, CallResult<Reply, messaging::Error>)>,
{
    const DISPATCH_CHANNEL_SIZE: usize = 1;
    let (dispatch_sender, dispatch_receiver) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
    let dispatch_sender = PollSender::new(dispatch_sender);
    let dispatch = dispatch(dispatch_receiver, requests_sink, responses_stream);
    (
        Client {
            dispatch_request_sender: dispatch_sender,
            id_factory: IdFactory::new(),
        },
        dispatch,
    )
}

#[derive(Debug, Clone)]
pub(crate) struct Client {
    dispatch_request_sender: PollSender<DispatchRequest>,
    id_factory: IdFactory,
}

impl Service<Call, Notification> for Client {
    type CallReply = Reply;
    type Error = Error;
    type CallFuture = CallFuture;
    type NotifyFuture = NotifyFuture;

    fn call(&mut self, call: Call) -> CallFuture {
        let mut this = &*self;
        this.call(call)
    }

    fn notify(&mut self, notif: Notification) -> NotifyFuture {
        let mut this = &*self;
        this.notify(notif)
    }
}

impl Service<Call, Notification> for &Client {
    type CallReply = Reply;
    type Error = Error;
    type CallFuture = CallFuture;
    type NotifyFuture = NotifyFuture;

    fn call(&mut self, call: Call) -> CallFuture {
        CallFuture::new(
            self.id_factory.create(),
            call,
            self.id_factory.clone(),
            self.dispatch_request_sender.clone(),
        )
    }

    fn notify(&mut self, notif: Notification) -> NotifyFuture {
        let id = self.id_factory.create();
        NotifyFuture {
            id,
            notification: Some(notif),
            dispatch_request_sender: self.dispatch_request_sender.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct IdFactory {
    current_id: Arc<AtomicU32>,
}

impl IdFactory {
    fn new() -> Self {
        Self {
            current_id: Arc::new(AtomicU32::new(1)),
        }
    }

    fn create(&self) -> RequestId {
        let id = self
            .current_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        RequestId::new(id)
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub(crate) struct CallFuture {
    request_id: RequestId,
    subject: Subject,
    id_factory: IdFactory,
    dispatch_request_sender: PollSender<DispatchRequest>,
    running: Option<CallFutureRunning>,
}

impl CallFuture {
    fn new(
        request_id: RequestId,
        call: Call,
        id_factory: IdFactory,
        dispatch_request_sender: PollSender<DispatchRequest>,
    ) -> Self {
        let subject = *call.subject();
        let running = CallFutureRunning::SendDispatchRequest(Some(call));
        Self {
            request_id,
            subject,
            id_factory,
            dispatch_request_sender,
            running: Some(running),
        }
    }

    pub(crate) fn cancel(&mut self) -> CancelFuture {
        match self.running.take() {
            Some(running) => running.cancel(
                self.subject,
                self.request_id,
                &self.id_factory,
                &self.dispatch_request_sender,
            ),
            None => CancelFuture(None),
        }
    }
}

impl ToRequestId for CallFuture {
    fn to_request_id(&self) -> RequestId {
        self.request_id
    }
}

impl Future for CallFuture {
    type Output = CallResult<Reply, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match &mut this.running {
            Some(running) => {
                let result = ready!(running.poll_run(
                    this.request_id,
                    &mut this.dispatch_request_sender,
                    cx
                ));
                this.running = None;
                Poll::Ready(result)
            }
            None => Poll::Pending,
        }
    }
}

impl Drop for CallFuture {
    fn drop(&mut self) {
        let cancel = self.cancel();
        // Spawn the cancel task if it's not already terminated.
        if !cancel.is_terminated() {
            task::spawn(cancel);
        }
    }
}

impl futures::future::FusedFuture for CallFuture {
    fn is_terminated(&self) -> bool {
        self.running.is_none()
    }
}

#[derive(Debug)]
enum CallFutureRunning {
    SendDispatchRequest(Option<Call>),
    WaitForResponse(oneshot::Receiver<CallResult<Reply, messaging::Error>>),
}

impl CallFutureRunning {
    fn poll_run(
        &mut self,
        id: RequestId,
        dispatch_request_sender: &mut PollSender<DispatchRequest>,
        cx: &mut Context<'_>,
    ) -> Poll<CallResult<Reply, Error>> {
        loop {
            match self {
                Self::SendDispatchRequest(call) => {
                    ready!(dispatch_request_sender.poll_reserve(cx))
                        .map_err(|_err| Error::DispatchTerminated)?;
                    let (response_sender, response_receiver) = oneshot::channel();
                    let call = match call.take() {
                        Some(call) => call,
                        // Theoretically should not occur. The only possible case that
                        // it could happen is if `send_item` fails and user polls the
                        // future once again after the error was returned.
                        None => break Poll::Pending,
                    };
                    dispatch_request_sender
                        .send_item(DispatchRequest::Call {
                            id,
                            call,
                            response_sender,
                        })
                        .map_err(|_err| Error::DispatchDroppedResponse)?;
                    *self = Self::WaitForResponse(response_receiver);
                }
                Self::WaitForResponse(response_receiver) => {
                    let reply = ready!(response_receiver.poll_unpin(cx))
                        .map_err(|_err| Error::DispatchDroppedResponse)?
                        .map_err(|err| err.map_err(Error::Messaging))?;
                    break Poll::Ready(Ok(reply));
                }
            }
        }
    }

    fn cancel(
        self,
        subject: Subject,
        call_id: RequestId,
        id_factory: &IdFactory,
        dispatch_request_sender: &PollSender<DispatchRequest>,
    ) -> CancelFuture {
        match self {
            // Nothing, no request has been sent yet.
            Self::SendDispatchRequest(..) => CancelFuture(None),
            Self::WaitForResponse(..) => {
                let cancel = Cancel::new(subject, call_id);
                let id = id_factory.create();
                let notification = DispatchRequest::Notification {
                    id,
                    notif: cancel.into(),
                };
                let mut sender = dispatch_request_sender.clone();
                let send_cancel = async move {
                    // This is a best effort function, so ignore the result, even if it is in error.
                    let _result = sender.send(notification).await;
                }
                .boxed();
                CancelFuture(Some(send_cancel))
            }
        }
    }
}

#[must_use = "futures do nothing until polled"]
pub struct CancelFuture(Option<BoxFuture<'static, ()>>);

impl Future for CancelFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(future) = &mut self.as_mut().0 {
            ready!(future.poll_unpin(cx));
            self.0 = None;
        }
        Poll::Ready(())
    }
}

impl FusedFuture for CancelFuture {
    fn is_terminated(&self) -> bool {
        self.0.is_none()
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub(crate) struct NotifyFuture {
    id: RequestId,
    notification: Option<Notification>,
    dispatch_request_sender: PollSender<DispatchRequest>,
}

impl ToRequestId for NotifyFuture {
    fn to_request_id(&self) -> RequestId {
        self.id
    }
}

impl Future for NotifyFuture {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let notif = match this.notification.take() {
            Some(notif) => notif,
            None => return Poll::Pending,
        };
        ready!(this.dispatch_request_sender.poll_reserve(cx))
            .map_err(|_err| Error::DispatchTerminated)?;
        this.dispatch_request_sender
            .send_item(DispatchRequest::Notification { id: this.id, notif })
            .map_err(|_err| Error::DispatchTerminated)?;
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("the client dispatch task is terminated")]
    DispatchTerminated,

    #[error("the client dispatch task has dropped the request response")]
    DispatchDroppedResponse,

    #[error(transparent)]
    Messaging(#[from] messaging::Error),
}

async fn dispatch<St, Si>(
    mut request_receiver: mpsc::Receiver<DispatchRequest>,
    requests_sink: Si,
    responses_stream: St,
) -> Result<(), Si::Error>
where
    Si: Sink<RequestWithId>,
    Si::Error: std::error::Error,
    St: Stream<Item = (RequestId, CallResult<Reply, messaging::Error>)>,
{
    let mut ongoing_call_requests = HashMap::new();
    let requests_sink = requests_sink;
    let responses_stream = responses_stream.fuse();
    pin!(responses_stream, requests_sink);

    loop {
        select! {
            Some(request) = request_receiver.recv() => {
                let (id, request) = match request {
                    DispatchRequest::Call {
                        id,
                        call,
                        response_sender,
                    } => {
                        trace!(%id, "registering a call request waiting for a response from the server");
                        ongoing_call_requests.insert(id, response_sender);
                        (id, call.into())
                    }
                    DispatchRequest::Notification{ id, notif } => (id, notif.into()),
                };
                requests_sink.send(RequestWithId::new(id, request)).await?;
            }
            Some((id, response)) = responses_stream.next() => {
                trace!(response = ?response, "received a call response from the server");
                if let Some(response_sender) = ongoing_call_requests.remove(&id) {
                    if let Err(response) = response_sender.send(response) {
                        trace!(response = ?response, "the client for a call request response has dropped, discarding response");
                    }
                }
            }
            else => {
                trace!("client dispatch is finished");
                break Ok(());
            }
        }

        // Cleanup ongoing call requests for which the client has dropped the channel.
        ongoing_call_requests.retain(|_id, response_sender| !response_sender.is_closed())
    }
}

#[derive(Debug)]
pub(crate) enum DispatchRequest {
    Call {
        id: RequestId,
        call: Call,
        response_sender: oneshot::Sender<CallResult<Reply, messaging::Error>>,
    },
    Notification {
        id: RequestId,
        notif: Notification,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        messaging::{CallTermination, Post, Reply, Request, Subject},
        service,
    };
    use assert_matches::assert_matches;
    use futures::future::{poll_immediate, BoxFuture};
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSendError;

    struct TestClient {
        requests_rx: mpsc::Receiver<RequestWithId>,
        responses_tx: mpsc::Sender<(RequestId, CallResult<Reply, messaging::Error>)>,
        client: Client,
        dispatch: BoxFuture<'static, Result<(), PollSendError<RequestWithId>>>,
    }

    impl TestClient {
        fn new() -> Self {
            let (requests_tx, requests_rx) = mpsc::channel(1);
            let (responses_tx, responses_rx) = mpsc::channel(1);
            let requests_sink = PollSender::new(requests_tx);
            let responses_stream = ReceiverStream::new(responses_rx);
            let (client, dispatch) = setup(responses_stream, requests_sink);
            Self {
                requests_rx,
                responses_tx,
                client,
                dispatch: dispatch.boxed(),
            }
        }
    }

    #[tokio::test]
    async fn test_client_drop_client_causes_terminated_error() {
        let mut test = TestClient::new();

        drop(test.dispatch);

        let res = test
            .client
            .call(Call::new(Subject::default()).with_formatted_value([1, 2, 3].into()))
            .await;
        assert_matches!(res, Err(CallTermination::Error(Error::DispatchTerminated)));
    }

    #[tokio::test]
    async fn test_client_drop_dispatch_task_after_call_causes_canceled_error() {
        let mut test = TestClient::new();

        let mut call = test
            .client
            .call(Call::new(Subject::default()).with_formatted_value([1, 2, 3, 4].into()));

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The dispatch is now waiting for the response. Drop the task.
        // It must cancel pending requests.
        drop(test.dispatch);
        assert_matches!(
            poll_immediate(call).await,
            Some(Err(CallTermination::Error(Error::DispatchDroppedResponse)))
        );
    }

    #[tokio::test]
    async fn test_client_post() {
        let mut test = TestClient::new();

        let notification_sent: Notification = Post::new(Subject::default())
            .with_formatted_value([1, 2, 3, 4].into())
            .into();
        let mut notify_future = test.client.notify(notification_sent.clone());

        assert_matches!(poll_immediate(&mut notify_future).await, Some(Ok(())));
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(),RequestId(1));
                assert_eq!(request.into_inner(), Request::Notification(notification_sent));
            }
        );
    }

    #[tokio::test]
    async fn test_client_call() {
        let mut test = TestClient::new();

        let call_sent = Call::new(Subject::default()).with_formatted_value([1, 2, 3, 4].into());
        let mut call_future = test.client.call(call_sent.clone());

        assert_matches!(poll_immediate(&mut call_future).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(),RequestId(1));
                assert_eq!(request.into_inner(), Request::Call(call_sent));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call_future).await, None);

        test.responses_tx
            .send((RequestId(1), Ok(Reply::new([5, 6, 7, 8].into()))))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(poll_immediate(&mut call_future).await, Some(Ok(reply)) => {
            assert_eq!(reply, Reply::new([5, 6, 7, 8].into()));
        });

        // Trying to cancel the future does nothing because it was terminated.
        call_future.cancel().await;
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);
        assert_matches!(poll_immediate(test.requests_rx.recv()).await, None);
    }

    #[tokio::test]
    async fn test_client_call_cancel() {
        let mut test = TestClient::new();

        let mut call_future = test
            .client
            .call(Call::new(Subject::default()).with_formatted_value([1, 2, 3, 4].into()));

        assert_matches!(poll_immediate(&mut call_future).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);
        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(), RequestId(1));
            }
        );

        // Cancel the call.
        assert_matches!(poll_immediate(call_future.cancel()).await, Some(()));

        // A cancellation of the call has been requested.
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);
        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(), RequestId(2));
                assert_eq!(
                    request.into_inner(),
                    Request::Notification(Cancel::new(Subject::default(), RequestId(1)).into())
                );
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call_future).await, None);
    }

    #[tokio::test]
    async fn test_client_call_cancel_on_future_drop() {
        let mut test = TestClient::new();

        let mut call_future = test
            .client
            .call(Call::new(Subject::default()).with_formatted_value([1, 2, 3, 4].into()));

        assert_matches!(poll_immediate(&mut call_future).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);
        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(), RequestId(1));
            }
        );

        drop(call_future);
        task::yield_now().await; // Yield to let the spawned cancel task execute.

        // A cancellation of the call has been requested.
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);
        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(), RequestId(2));
                assert_eq!(
                    request.into_inner(),
                    Request::Notification(Cancel::new(Subject::default(), RequestId(1)).into())
                );
            }
        );
    }

    #[tokio::test]
    async fn test_client_call_error_response() {
        let mut test = TestClient::new();

        let call_sent = Call::new(Subject::default()).with_formatted_value([1, 2, 3, 4].into());
        let mut call_future = test.client.call(call_sent.clone());

        assert_matches!(poll_immediate(&mut call_future).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(), RequestId(1));
                assert_eq!(request.into_inner(), Request::Call(call_sent));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call_future).await, None);

        test.responses_tx
            .send((
                RequestId(1),
                Err(CallTermination::Error(messaging::Error(
                    "some error".to_owned(),
                ))),
            ))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(
            poll_immediate(&mut call_future).await,
            Some(Err(CallTermination::Error(Error::Messaging(service::Error(err))))) => {
                assert_eq!(err, "some error");
            }
        );
    }

    #[tokio::test]
    async fn test_client_call_canceled_response() {
        let mut test = TestClient::new();

        let call_sent = Call::new(Subject::default()).with_formatted_value([1, 2, 3, 4].into());
        let mut call_future = test.client.call(call_sent.clone());

        assert_matches!(poll_immediate(&mut call_future).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(), RequestId(1));
                assert_eq!(request.into_inner(), Request::Call(call_sent));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call_future).await, None);

        test.responses_tx
            .send((RequestId(1), Err(CallTermination::Canceled)))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(
            poll_immediate(&mut call_future).await,
            Some(Err(CallTermination::Canceled))
        );
    }

    #[tokio::test]
    async fn test_client_call_ignores_responses_of_other_id() {
        let mut test = TestClient::new();

        let call_sent = Call::new(Subject::default()).with_formatted_value([1, 2, 3, 4].into());
        let mut call_future = test.client.call(call_sent.clone());

        assert_matches!(poll_immediate(&mut call_future).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(request)) => {
                assert_eq!(request.id(), RequestId(1));
                assert_eq!(request.into_inner(), Request::Call(call_sent));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call_future).await, None);

        // Send a response of request id = 2.
        test.responses_tx
            .send((RequestId(2), Ok(Reply::new([5, 6, 7, 8].into()))))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call_future).await, None);

        // Send a response of request id = 1.
        let reply_sent = Reply::new([9, 10, 11, 12].into());
        test.responses_tx
            .send((RequestId(1), Ok(reply_sent.clone())))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(poll_immediate(&mut call_future).await, Some(Ok(reply)) => {
            assert_eq!(reply, reply_sent);
        });
    }

    #[tokio::test]
    async fn test_client_sink_error_stops_dispatch_task() {
        let mut test = TestClient::new();

        // Drop the sink receiver, this will cause errors from the sender.
        drop(test.requests_rx);

        let call = test.client.call(Call::new(Subject::default()));

        // The call cannot finish until dispatch is run.
        assert_matches!(poll_immediate(call).await, None);
        assert_matches!(poll_immediate(test.dispatch).await, Some(Err(_)));
    }
}
