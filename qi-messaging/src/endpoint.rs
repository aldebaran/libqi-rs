use crate::{
    capabilities::CapabilitiesMap,
    message::{self, Address, Id, Message},
    Call, Capabilities, Client, Error, Event, Notification, Post,
};
use bytes::Bytes;
use erased_serde::Serialize;
use futures::{
    future::{abortable, AbortHandle, Abortable, Aborted},
    ready,
    stream::{self, FusedStream, FuturesUnordered},
    Sink, SinkExt, Stream, StreamExt,
};
use pin_project_lite::pin_project;
use qi_format as format;
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
use tokio_util::sync::{CancellationToken, PollSender, WaitForCancellationFutureOwned};
use tower::{Service, ServiceExt};
use tracing::debug;

pub fn open<'a, M, Svc>(
    messages: M,
    service: Svc,
) -> (
    impl Stream<Item = Message> + 'a,
    broadcast::Receiver<Notification<'static>>,
    Client,
)
where
    M: Stream<Item = Message> + Unpin + 'a,
    Svc: Service<Call<Bytes>, Response = Bytes, Error = Error> + Sink<Post<Bytes>> + Unpin + 'a,
    Svc::Future: 'a,
{
    const FIRST_ID: u32 = 1;

    let (requests_sender, requests) = mpsc::channel(1);
    let (notifications_sender, notifications) = broadcast::channel(1);

    let dispatch = Endpoint {
        service,
        messages: messages.fuse(),
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
    let client = Client::new(PollSender::new(requests_sender));

    (messages, notifications, client)
}

pub(crate) enum ClientRequest {
    Call {
        address: message::Address,
        args: Box<dyn Serialize + Send>,
        cancel_token: CancellationToken,
        response_sender: oneshot::Sender<Result<Bytes, Error>>,
    },
    Post {
        address: message::Address,
        value: Box<dyn Serialize + Send>,
    },
}

#[derive(Debug)]
pub struct Endpoint<M, Svc, F> {
    messages: M,
    service: Svc,
    id: AtomicU32,
    requests: mpsc::Receiver<ClientRequest>,
    notifications: broadcast::Sender<Notification<'static>>,
    service_call_futures: FuturesUnordered<ServiceCallFuture<F>>,
    client_calls: FuturesUnordered<ClientCallFuture>,
}

impl<M, Svc> Endpoint<M, Svc, Svc::Future>
where
    M: FusedStream<Item = Message> + Unpin,
    Svc: Service<Call<Bytes>, Response = Bytes, Error = Error> + Sink<Post<Bytes>> + Unpin,
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
                        Ok(reply) => Message::reply(id, address).set_body(reply).build(),
                        Err(Error::Canceled) => Message::canceled(id, address).build(),
                        Err(err) => match Message::error(id, address, &err.to_string()) {
                            Ok(builder) => builder.build(),
                            Err(err) => Message::error(id, address,
                                &format!("the call request has terminated with an error, \
                                    but the serialization of the error message failed: {err}")).unwrap().build()
                        }
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
            args: message.body,
        };
        let ready = self.service.ready().await;
        let service_call_future = match ready {
            Err(error) => ServiceCallFuture::ReadyError {
                id,
                address,
                error: Some(error),
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
            client_call.handle_response(Ok(message.body));
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
            client_call.handle_response(Err(Error::Other(description.into())));
        }
    }

    async fn handle_post(&mut self, message: Message) {
        let post = Post {
            address: message.address,
            value: message.body,
        };
        let _res = self.service.send(post).await;
    }

    fn handle_event(&self, message: Message) {
        let event = Event {
            address: message.address,
            body: message.body,
        };
        let notification = Notification::Event(event);
        let _res = self.notifications.send(notification);
    }

    fn handle_capabilities(&self, message: Message) {
        if let Ok(map) = message.deserialize_body::<CapabilitiesMap<'_>>() {
            let capabilities = Capabilities {
                address: message.address,
                capabilities: map.into_iter().map(|(k, v)| (k, v.into_owned())).collect(),
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
            client_call.handle_response(Err(Error::Canceled));
        }
    }

    fn handle_client_request(&self, request: ClientRequest) -> Option<Message> {
        let id = self.new_id();
        match request {
            ClientRequest::Call {
                address,
                args,
                response_sender,
                cancel_token,
            } => {
                let body = match format::to_bytes(&args) {
                    Ok(body) => body,
                    Err(err) => {
                        let _res = response_sender.send(Err(Error::Other(err.into())));
                        return None;
                    }
                };
                let client_call = ClientCallFuture {
                    id,
                    cancelled: cancel_token.cancelled_owned(),
                    waker: None,
                    response_sender: Some(response_sender),
                };
                self.client_calls.push(client_call);
                let message = Message::call(id, address).set_body(body).build();
                Some(message)
            }
            ClientRequest::Post { address, value } => {
                let body = format::to_bytes(&value).ok()?;
                let message = Message::post(id, address).set_body(body).build();
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
            error: Option<Error>,
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
    F: std::future::Future<Output = Result<Bytes, Error>>,
{
    type Output = (Id, Address, Result<Bytes, Error>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            ServiceCallFutureProj::ReadyError { id, address, error } => match error.take() {
                Some(error) => {
                    let value = (*id, *address, Err(error));
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
                    Err(Aborted) => Err(Error::Canceled),
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
        response_sender: Option<oneshot::Sender<Result<Bytes, Error>>>,
    }
}

impl ClientCallFuture {
    fn handle_response(self: Pin<&mut Self>, response: Result<Bytes, Error>) {
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
    use futures::{future, sink, SinkExt};
    use std::sync::Arc;
    use tokio::sync::Barrier;
    use tokio_test::{
        assert_pending, assert_ready, assert_ready_eq, stream_mock::StreamMockBuilder, task,
    };

    struct ServiceSinkPair<Svc, Sink>(Svc, Sink);

    impl<Req, Svc, Sink> tower::Service<Req> for ServiceSinkPair<Svc, Sink>
    where
        Svc: tower::Service<Req>,
    {
        type Response = Svc::Response;
        type Error = Svc::Error;
        type Future = Svc::Future;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.0.poll_ready(cx)
        }

        fn call(&mut self, req: Req) -> Self::Future {
            self.0.call(req)
        }
    }

    impl<T, Svc, Sink> futures::Sink<T> for ServiceSinkPair<Svc, Sink>
    where
        Svc: Unpin,
        Sink: futures::Sink<T> + Unpin,
    {
        type Error = Sink::Error;

        fn poll_ready(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            self.1.poll_ready_unpin(cx)
        }

        fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
            self.1.start_send_unpin(item)
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            self.1.poll_flush_unpin(cx)
        }

        fn poll_close(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            self.1.poll_close_unpin(cx)
        }
    }

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
                Ok(Bytes::new())
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

        let (messages, _, _) = open(messages, ServiceSinkPair(service, posts));
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

        let (messages, _, _) = open(messages, ServiceSinkPair(service, posts));
        let mut messages = task::spawn(messages);

        // One cancelled message is produced.
        assert_ready_eq!(
            messages.poll_next(),
            Some(Message::canceled(Id(1), Address::default()).build())
        );
        assert_ready_eq!(messages.poll_next(), None);
    }
}
