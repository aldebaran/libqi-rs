use crate::{
    capabilities, format,
    message::{self, Address, Id, Message},
};
use futures::{
    future::{abortable, AbortHandle, Abortable, Aborted},
    ready,
    stream::{self, FusedStream, FuturesUnordered},
    FutureExt, Sink, SinkExt, Stream, StreamExt,
};
use pin_project_lite::pin_project;
use std::{
    fmt::Debug,
    pin::Pin,
    sync::atomic::AtomicU32,
    task::{Context, Poll, Waker},
};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tokio_util::sync::{
    CancellationToken, DropGuard, PollSendError, PollSender, WaitForCancellationFutureOwned,
};
use tower::{Service, ServiceExt};
use tracing::debug;

pub fn open<'a, M, Svc, P>(
    messages: M,
    service: Svc,
    posts: P,
) -> (impl Stream<Item = Message> + 'a, Client)
where
    M: Stream<Item = Message> + 'a,
    Svc: Service<Call, Response = Reply, Error = Abandon> + 'a,
    P: Sink<Post> + 'a,
    Svc::Future: 'a,
{
    const FIRST_ID: u32 = 1;

    let (requests_sender, requests) = mpsc::channel(1);
    let (notifications_sender, notifications) = broadcast::channel(1);

    let dispatch = Endpoint {
        service,
        posts: Box::pin(posts),
        messages: Box::pin(messages.fuse()),
        id: AtomicU32::new(FIRST_ID),
        requests,
        notifications: notifications_sender,
        service_call_futures: FuturesUnordered::new(),
        client_calls: FuturesUnordered::new(),
    };
    let messages = stream::unfold(dispatch, |mut endpoint| async move {
        let message = endpoint.next_message().await;
        message.map(move |message| (message, endpoint))
    });
    let client = Client {
        requests: PollSender::new(requests_sender),
        notifications,
    };

    (messages, client)
}

#[derive(Debug)]
pub struct Client {
    requests: PollSender<Request>,
    notifications: broadcast::Receiver<Notification>,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            requests: self.requests.clone(),
            notifications: self.notifications.resubscribe(),
        }
    }
}

impl Service<Call> for Client {
    type Response = Reply;
    type Error = CallError;
    type Future = CallFuture;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_reserve(cx)?.map(Ok)
    }

    fn call(&mut self, call: Call) -> CallFuture {
        let (response_sender, response_receiver) = oneshot::channel();
        let cancel_token = CancellationToken::new();
        let request = Request::Call {
            call,
            cancel_token: cancel_token.clone(),
            response_sender,
        };
        let inner = match self.requests.send_item(request) {
            Ok(()) => CallInnerFuture::RequestSent {
                response_receiver,
                drop_guard: Some(cancel_token.drop_guard()),
            },
            Err(_send_err) => CallInnerFuture::SendError,
        };
        CallFuture(inner)
    }
}

impl Sink<Post> for Client {
    type Error = CallError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_ready_unpin(cx)?.map(Ok)
    }

    fn start_send(mut self: Pin<&mut Self>, post: Post) -> Result<(), Self::Error> {
        Ok(self.requests.start_send_unpin(Request::Post(post))?)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_flush_unpin(cx)?.map(Ok)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_close_unpin(cx)?.map(Ok)
    }
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Call {
    pub(crate) address: message::Address,
    pub(crate) value: format::Value,
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Reply {
    pub(crate) value: format::Value,
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Post {
    pub(crate) address: message::Address,
    pub(crate) value: format::Value,
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Event {
    pub(crate) address: message::Address,
    pub(crate) value: format::Value,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Capabilities {
    pub(crate) address: message::Address,
    pub(crate) capabilities: capabilities::CapabilitiesMap,
}

#[derive(Clone, PartialEq, Eq, Debug, derive_more::From, serde::Serialize, serde::Deserialize)]
pub enum Notification {
    Event(Event),
    Capabilities(Capabilities),
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    derive_more::From,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Abandon {
    Error(String),
    Canceled,
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub struct CallFuture(CallInnerFuture);

impl CallFuture {
    pub fn cancel(&mut self) {
        if let CallInnerFuture::RequestSent { drop_guard, .. } = &mut self.0 {
            if let Some(guard) = drop_guard.take() {
                guard.disarm().cancel()
            }
        }
    }
}

impl std::future::Future for CallFuture {
    type Output = Result<Reply, CallError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.0 {
            CallInnerFuture::SendError => Poll::Ready(Err(CallError::Disconnected)),
            CallInnerFuture::RequestSent {
                response_receiver, ..
            } => {
                let reply = ready!(response_receiver.poll_unpin(cx)?)?;
                Poll::Ready(Ok(reply))
            }
        }
    }
}

#[derive(Debug)]
enum CallInnerFuture {
    SendError,
    RequestSent {
        response_receiver: oneshot::Receiver<Result<Reply, Abandon>>,
        drop_guard: Option<DropGuard>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("{0}")]
    Message(String),

    #[error("client is disconnected")]
    Disconnected,

    #[error("canceled")]
    Canceled,
}

impl From<PollSendError<Request>> for CallError {
    fn from(_err: PollSendError<Request>) -> Self {
        Self::Disconnected
    }
}

impl From<oneshot::error::RecvError> for CallError {
    fn from(_err: oneshot::error::RecvError) -> Self {
        Self::Disconnected
    }
}

impl From<Abandon> for CallError {
    fn from(failure: Abandon) -> Self {
        match failure {
            Abandon::Error(err) => Self::Message(err),
            Abandon::Canceled => Self::Canceled,
        }
    }
}

#[derive(Debug)]
enum Request {
    Call {
        call: Call,
        cancel_token: CancellationToken,
        response_sender: oneshot::Sender<Result<Reply, Abandon>>,
    },
    Post(Post),
}

#[derive(Debug)]
pub struct Endpoint<M, Svc, P, F> {
    messages: M,
    service: Svc,
    posts: P,
    id: AtomicU32,
    requests: mpsc::Receiver<Request>,
    notifications: broadcast::Sender<Notification>,
    service_call_futures: FuturesUnordered<ServiceCallFuture<F>>,
    client_calls: FuturesUnordered<ClientCallFuture>,
}

impl<M, Svc, P> Endpoint<M, Svc, P, Svc::Future>
where
    M: FusedStream<Item = Message> + Unpin,
    Svc: Service<Call, Response = Reply, Error = Abandon>,
    P: Sink<Post> + Unpin,
{
    async fn next_message(&mut self) -> Option<Message> {
        loop {
            select! {
                // Receive a message from the stream.
                Some(message) = self.messages.next(), if !self.messages.is_terminated() => {
                    self.handle_message(message).await
                }
                // Receive a request from a client.
                Some(request) = self.requests.recv() => {
                    if let Some(message) = self.handle_client_request(request) {
                        break Some(message)
                    }
                }
                // Try finishing service calls.
                Some((id, address, result)) = self.service_call_futures.next(),
                    if !self.service_call_futures.is_terminated() => {
                    let message = match result {
                        Ok(reply) => Message::reply(id, address).set_body(reply.value).build(),
                        Err(Abandon::Error(err)) => match Message::error(id, address, &err) {
                            Ok(builder) => builder.build(),
                            Err(err) => Message::error(id, address,
                                &format!("the call request has terminated with an error, \
                                    but the serialization of the error message failed: {err}")).unwrap().build()
                        }
                        Err(Abandon::Canceled) => Message::canceled(id, address).build(),
                    };
                    break Some(message)
                }
                // Try finishing client calls.
                Some(_res) = self.client_calls.next(), if !self.client_calls.is_terminated() => {
                    // Nothing to do, either the call completed because we got a response, or it was
                    // cancelled by the client.
                }
                // No more work to do, no more message will be produced.
                else => {
                    break None
                }
            }
        }
    }

    async fn handle_message(&mut self, message: Message) {
        match message.ty {
            message::Type::Call => self.handle_call(message).await,
            message::Type::Reply => self.handle_reply(message),
            message::Type::Error => self.handle_error(message),
            message::Type::Post => self.handle_post(message).await,
            message::Type::Event => self.handle_event(message),
            message::Type::Capabilities => self.handle_capabilities(message),
            message::Type::Cancel => self.handle_cancel(message),
            message::Type::Canceled => self.handle_canceled(message),
        }
    }

    async fn handle_call(&mut self, message: Message) {
        let id = message.id;
        let address = message.address;
        let call = Call {
            address,
            value: message.body,
        };
        let ready = self.service.ready().await;
        let service_call_future = match ready {
            Err(abandon) => ServiceCallFuture::ReadyError {
                id,
                address,
                abandon: Some(abandon),
            },
            Ok(service) => {
                let (future, abort) = abortable(service.call(call));
                ServiceCallFuture::Call {
                    id,
                    address,
                    abort,
                    future,
                }
            }
        };
        self.service_call_futures.push(service_call_future);
    }

    fn handle_reply(&mut self, message: Message) {
        if let Some(client_call) = self.find_client_call_mut(message.id) {
            client_call.handle_response(Ok(Reply {
                value: message.body,
            }));
        }
    }

    fn handle_error(&mut self, message: Message) {
        if let Some(client_call) = self.find_client_call_mut(message.id) {
            let description = match message.deserialize_error_description() {
                Ok(description) => description,
                Err(err) => format!(
                    "the call request has terminated with an error, \
                    but the deserialization of the error message failed: {err}"
                ),
            };
            client_call.handle_response(Err(Abandon::Error(description)));
        }
    }

    async fn handle_post(&mut self, message: Message) {
        let post = Post {
            address: message.address,
            value: message.body,
        };
        let _res = self.posts.send(post).await;
    }

    fn handle_event(&self, message: Message) {
        let event = Event {
            address: message.address,
            value: message.body,
        };
        let notification = Notification::Event(event);
        let _res = self.notifications.send(notification);
    }

    fn handle_capabilities(&self, message: Message) {
        if let Ok(map) = message.deserialize_body() {
            let capabilities = Capabilities {
                address: message.address,
                capabilities: map,
            };
            let notification = Notification::Capabilities(capabilities);
            let _res = self.notifications.send(notification);
        }
    }

    fn handle_cancel(&mut self, message: Message) {
        let id: Id = match message.deserialize_body() {
            Ok(id) => id,
            Err(err) => {
                debug!(
                    error = &err as &dyn std::error::Error,
                    "failed to deserialize the body of a cancel message \
                    as a call id, discarding message"
                );
                return;
            }
        };
        if let Some(service_call_future) = Pin::new(&mut self.service_call_futures)
            .iter_pin_mut()
            .find(|future| future.id() == id)
        {
            service_call_future.cancel();
        }
    }

    fn handle_canceled(&mut self, message: Message) {
        if let Some(client_call) = self.find_client_call_mut(message.id) {
            client_call.handle_response(Err(Abandon::Canceled));
        }
    }

    fn handle_client_request(&self, request: Request) -> Option<Message> {
        let id = self.new_id();
        match request {
            Request::Call {
                call,
                response_sender,
                cancel_token,
            } => {
                let client_call = ClientCallFuture {
                    id,
                    cancelled: cancel_token.cancelled_owned(),
                    waker: None,
                    response_sender: Some(response_sender),
                };
                self.client_calls.push(client_call);
                let message = Message::call(id, call.address).set_body(call.value).build();
                Some(message)
            }
            Request::Post(post) => {
                let message = Message::post(id, post.address).set_body(post.value).build();
                Some(message)
            }
        }
    }

    fn new_id(&self) -> Id {
        use std::sync::atomic::Ordering;
        Id(self.id.fetch_add(1, Ordering::SeqCst))
    }

    fn find_client_call_mut(&mut self, id: Id) -> Option<Pin<&mut ClientCallFuture>> {
        Pin::new(&mut self.client_calls)
            .iter_pin_mut()
            .find(|call| call.id == id)
    }
}

pin_project! {
    #[derive(Debug)]
    #[project = ServiceCallFutureProj]
    enum ServiceCallFuture<F> {
        ReadyError {
            id: Id,
            address: Address,
            abandon: Option<Abandon>,
        },
        Call {
            id: Id,
            address: Address,
            abort: AbortHandle,
            #[pin]
            future: Abortable<F>,
        },
    }
}

impl<F> ServiceCallFuture<F> {
    fn id(&self) -> Id {
        match self {
            Self::ReadyError { id, .. } => *id,
            Self::Call { id, .. } => *id,
        }
    }

    fn cancel(&self) {
        if let Self::Call { abort, .. } = self {
            abort.abort();
        }
    }
}

impl<F> std::future::Future for ServiceCallFuture<F>
where
    F: std::future::Future<Output = Result<Reply, Abandon>>,
{
    type Output = (Id, Address, Result<Reply, Abandon>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            ServiceCallFutureProj::ReadyError {
                id,
                address,
                abandon,
            } => match abandon.take() {
                Some(abandon) => {
                    let value = (*id, *address, Err(abandon));
                    Poll::Ready(value)
                }
                None => Poll::Pending,
            },
            ServiceCallFutureProj::Call {
                id,
                address,
                future,
                ..
            } => {
                let result = match ready!(future.poll(cx)) {
                    Ok(result) => result,
                    Err(Aborted) => Err(Abandon::Canceled),
                };
                Poll::Ready((*id, *address, result))
            }
        }
    }
}

pin_project! {
    #[derive(Debug)]
    struct ClientCallFuture {
        id: Id,
        #[pin]
        cancelled: WaitForCancellationFutureOwned,
        waker: Option<Waker>,
        response_sender: Option<oneshot::Sender<Result<Reply, Abandon>>>,
    }
}

impl ClientCallFuture {
    fn handle_response(self: Pin<&mut Self>, response: Result<Reply, Abandon>) {
        let this = self.project();
        if let Some(sender) = this.response_sender.take() {
            let _res = sender.send(response);
            if let Some(waker) = this.waker.take() {
                waker.wake();
            }
        }
    }
}

impl std::future::Future for ClientCallFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if this.response_sender.is_some() {
            match this.cancelled.poll(cx) {
                Poll::Ready(()) => {
                    // The future has been canceled by the client, terminate the call.
                    Poll::Ready(())
                }
                Poll::Pending => {
                    *this.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        } else {
            // The response was received and sent into the channel, the client call is finished.
            Poll::Ready(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{future, sink};
    use std::sync::Arc;
    use tokio::sync::Barrier;
    use tokio_test::{
        assert_pending, assert_ready, assert_ready_eq, stream_mock::StreamMockBuilder, task,
    };

    #[tokio::test]
    async fn test_concurrent_service_calls() {
        // N number of service concurrent calls.
        const SERVICE_CONCURRENT_CALLS: usize = 5;

        // Create a barrier that will unlock when all concurrent calls (N) and the test (+1) are
        // waiting for it.
        let service_wait_barrier = Arc::new(Barrier::new(SERVICE_CONCURRENT_CALLS + 1));
        let service = tower::service_fn(|_call| {
            let wait_barrier = Arc::clone(&service_wait_barrier);
            async move {
                let _res = wait_barrier.wait().await;
                Ok(Reply::default())
            }
        });

        // Send N call messages to the endpoint.
        let mut messages_builder = StreamMockBuilder::new();
        for _ in 0..SERVICE_CONCURRENT_CALLS {
            messages_builder =
                messages_builder.next(Message::call(Id::default(), Address::default()).build());
        }
        let messages = messages_builder.build();
        let posts = sink::drain();

        let (messages, _) = open(messages, service, posts);
        let mut messages = task::spawn(messages);

        // No message is produced yet, the service is blocked.
        assert_pending!(messages.poll_next());

        // Block the test on the barrier, which can only be unlocked if the service has been called
        // N times already, without waiting for each call to terminate.
        let mut service_unlock = task::spawn(service_wait_barrier.wait());
        assert_ready!(service_unlock.poll());

        // Some reply messages are produced.
        for _ in 0..SERVICE_CONCURRENT_CALLS {
            assert_ready_eq!(
                messages.poll_next(),
                Some(Message::reply(Id::default(), Address::default()).build())
            );
        }
        // And then the stream is over.
        assert_ready_eq!(messages.poll_next(), None);
    }

    #[tokio::test]
    async fn test_service_call_cancel() {
        // A service that never finishes.
        let service = tower::service_fn(|_call| future::pending());
        let messages = StreamMockBuilder::new()
            .next(Message::call(Id(1), Address::default()).build())
            .next(Message::cancel(Id(2), Address::default(), Id(1)).build())
            .build();
        let posts = sink::drain();

        let (messages, _) = open(messages, service, posts);
        let mut messages = task::spawn(messages);

        // One cancelled message is produced.
        assert_ready_eq!(
            messages.poll_next(),
            Some(Message::canceled(Id(1), Address::default()).build())
        );
        assert_ready_eq!(messages.poll_next(), None);
    }
}
