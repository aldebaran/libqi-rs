use crate::{
    message::{self, Address, Id, Message},
    Call, Client, Error, Event, Post, Service,
};
use bytes::Bytes;
use futures::{
    future::{abortable, AbortHandle, Aborted, BoxFuture},
    ready,
    stream::{self, FusedStream, FuturesUnordered},
    FutureExt, Stream, StreamExt,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    pin::Pin,
    sync::atomic::AtomicU32,
    task::{Context, Poll},
};
use tokio::{
    select,
    sync::{mpsc, oneshot},
};
use tokio_util::sync::{CancellationToken, DropGuard, PollSender};
use tracing::debug;

pub fn endpoint<'a, M, Svc>(messages: M, service: Svc) -> (Endpoint<'a, M, Svc>, Client)
where
    Svc: Service,
{
    const FIRST_ID: u32 = 1;

    let (requests_sender, requests) = mpsc::channel(1);
    let endpoint = Endpoint {
        messages,
        service,
        id: AtomicU32::new(FIRST_ID),
        requests,
        service_call_futures: FuturesUnordered::new(),
        service_futures: FuturesUnordered::new(),
        client_call_responses: HashMap::new(),
        client_call_cancellations: FuturesUnordered::new(),
    };
    let client = Client::new(PollSender::new(requests_sender));
    (endpoint, client)
}

#[derive(Debug)]
pub(crate) enum ClientRequest {
    Call {
        call: Call,
        cancel_token: CancellationToken,
        response_sender: oneshot::Sender<Result<Bytes, Error>>,
    },
    Post(Post),
    Event(Event),
}

#[derive(Debug)]
pub struct Endpoint<'a, M, Svc> {
    messages: M,
    service: Svc,
    id: AtomicU32,
    requests: mpsc::Receiver<ClientRequest>,
    service_call_futures: FuturesUnordered<ServiceCallFuture<'a>>,
    service_futures: FuturesUnordered<BoxFuture<'a, Result<(), Error>>>,
    client_call_responses: HashMap<Id, (oneshot::Sender<Result<Bytes, Error>>, DropGuard)>,
    client_call_cancellations: FuturesUnordered<BoxFuture<'static, (Id, Address)>>,
}

impl<'a, 'r, M, Svc> Endpoint<'a, M, Svc>
where
    M: FusedStream<Item = Message> + Unpin,
    Svc: Service + 'a,
{
    pub async fn next_message(&mut self) -> Option<Message> {
        loop {
            select! {
                // Receive a dispatched message.
                Some(message) = self.messages.next(), if !self.messages.is_terminated() => {
                    self.handle_message(message)
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
                // Try finishing service posts/events.
                Some(_result) = self.service_futures.next(), if !self.service_futures.is_terminated() => {
                    // nothing, failures to post or send events are not handled.
                }
                // Try finishing client call cancellations. A cancellation means that the client
                // future was dropped, which can occur before or after we've received the response.
                // If no response was received before the cancellation, we send a cancel message, otherwise we do nothing.
                Some((call_id, address)) = self.client_call_cancellations.next(), if !self.client_call_cancellations.is_terminated() => {
                    if self.client_call_responses.contains_key(&call_id) {
                        let id = self.new_id();
                        let message = Message::cancel(id, address, call_id).build();
                        break Some(message);
                    }
                }
                // No more work to do, no more message will be produced.
                else => {
                    break None
                }
            }
        }
    }

    pub fn into_messages_stream(self) -> impl Stream<Item = Message> + FusedStream + 'a
    where
        M: 'a,
        Svc: 'a,
    {
        stream::unfold(self, |mut ep| async move {
            ep.next_message().await.map(|msg| (msg, ep))
        })
    }

    fn handle_message(&mut self, message: Message) {
        match message.ty {
            message::Type::Call => self.handle_call(message),
            message::Type::Reply => self.handle_reply(message),
            message::Type::Error => self.handle_error(message),
            message::Type::Post => self.handle_post(message),
            message::Type::Event => self.handle_event(message),
            message::Type::Capabilities => {} // unhandled message
            message::Type::Cancel => self.handle_cancel(message),
            message::Type::Canceled => self.handle_canceled(message),
        }
    }

    fn handle_call(&mut self, message: Message) {
        let id = message.id;
        let address = message.address;
        let call = Call {
            address,
            value: message.body,
        };
        let (future, abort) = abortable(self.service.call(call));
        self.service_call_futures.push(ServiceCallFuture {
            id,
            address,
            abort,
            future: future.boxed(),
        });
    }

    fn handle_reply(&mut self, message: Message) {
        self.send_client_call_response(&message.id, Ok(message.body));
    }

    fn handle_error(&mut self, message: Message) {
        let description = match message.deserialize_error_description() {
            Ok(description) => description,
            Err(err) => format!(
                "the call request has terminated with an error, \
                    but the deserialization of the error message failed: {err}"
            ),
        };
        self.send_client_call_response(&message.id, Err(Error::Message(description)));
    }

    fn handle_post(&mut self, message: Message) {
        let post = Post {
            address: message.address,
            value: message.body,
        };
        let post = self.service.post(post);
        self.service_futures.push(post.boxed());
    }

    fn handle_event(&mut self, message: Message) {
        let event = Event {
            address: message.address,
            value: message.body,
        };
        let event = self.service.event(event);
        self.service_futures.push(event.boxed());
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
        self.send_client_call_response(&message.id, Err(Error::Canceled));
    }

    fn handle_client_request(&mut self, request: ClientRequest) -> Option<Message> {
        let id = self.new_id();
        match request {
            ClientRequest::Call {
                call,
                response_sender,
                cancel_token,
            } => {
                self.client_call_responses
                    .insert(id, (response_sender, cancel_token.clone().drop_guard()));
                let address = call.address;
                self.client_call_cancellations.push(
                    async move {
                        cancel_token.cancelled_owned().await;
                        (id, address)
                    }
                    .boxed(),
                );
                let message = Message::call(id, address).set_body(call.value).build();
                Some(message)
            }
            ClientRequest::Post(post) => {
                let message = Message::post(id, post.address).set_body(post.value).build();
                Some(message)
            }
            ClientRequest::Event(event) => {
                let message = Message::event(id, event.address)
                    .set_body(event.value)
                    .build();
                Some(message)
            }
        }
    }

    fn send_client_call_response(&mut self, id: &Id, response: Result<Bytes, Error>) {
        if let Some((sender, _cancel)) = self.client_call_responses.remove(id) {
            let _res = sender.send(response);
        }
    }

    fn new_id(&self) -> Id {
        use std::sync::atomic::Ordering;
        Id(self.id.fetch_add(1, Ordering::SeqCst))
    }
}

struct ServiceCallFuture<'a> {
    id: Id,
    address: Address,
    abort: AbortHandle,
    future: BoxFuture<'a, Result<Result<Bytes, Error>, Aborted>>,
}

impl ServiceCallFuture<'_> {
    fn id(&self) -> Id {
        self.id
    }

    fn cancel(&self) {
        self.abort.abort()
    }
}

impl std::fmt::Debug for ServiceCallFuture<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceCallFuture")
            .field("id", &self.id)
            .field("address", &self.address)
            .finish()
    }
}

impl std::future::Future for ServiceCallFuture<'_> {
    type Output = (Id, Address, Result<Bytes, Error>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let result = match ready!(this.future.poll_unpin(cx)) {
            Ok(result) => result,
            Err(Aborted) => Err(Error::Canceled),
        };
        Poll::Ready((this.id, this.address, result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future;
    use std::sync::Arc;
    use tokio::sync::Barrier;
    use tokio_test::{assert_pending, assert_ready, assert_ready_eq, task};

    #[tokio::test]
    async fn test_concurrent_service_calls() {
        struct BlockingService(Arc<Barrier>);
        impl Service for BlockingService {
            fn call(&self, _call: Call) -> BoxFuture<'static, Result<Bytes, Error>> {
                let barrier = Arc::clone(&self.0);
                async move {
                    let _res = barrier.wait().await;
                    Ok(Bytes::new())
                }
                .boxed()
            }
            fn post(&self, _: Post) -> BoxFuture<'static, Result<(), Error>> {
                future::ok(()).boxed()
            }
            fn event(&self, _: Event) -> BoxFuture<'static, Result<(), Error>> {
                future::ok(()).boxed()
            }
        }

        // N number of service concurrent calls.
        const SERVICE_CONCURRENT_CALLS: usize = 5;

        // Create a barrier that will unlock when all concurrent calls (N) and the test (+1) are
        // waiting for it.
        let service_wait_barrier = Arc::new(Barrier::new(SERVICE_CONCURRENT_CALLS + 1));

        // Send N call messages to the endpoint.
        let messages = stream::iter(
            (0..SERVICE_CONCURRENT_CALLS)
                .map(|_id| Message::call(Id::default(), Address::default()).build()),
        )
        .fuse();

        let (endpoint, _) = endpoint(messages, BlockingService(Arc::clone(&service_wait_barrier)));
        let mut messages = task::spawn(endpoint.into_messages_stream());

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
        struct PendingService;
        impl Service for PendingService {
            fn call(&self, _call: Call) -> BoxFuture<'static, Result<Bytes, Error>> {
                future::pending().boxed()
            }
            fn post(&self, _: Post) -> BoxFuture<'static, Result<(), Error>> {
                future::ok(()).boxed()
            }
            fn event(&self, _: Event) -> BoxFuture<'static, Result<(), Error>> {
                future::ok(()).boxed()
            }
        }

        let messages = stream::iter([
            Message::call(Id(1), Address::default()).build(),
            Message::cancel(Id(2), Address::default(), Id(1)).build(),
        ])
        .fuse();

        let (endpoint, _) = endpoint(messages, PendingService);
        let mut messages = task::spawn(endpoint.into_messages_stream());

        // One cancelled message is produced.
        assert_ready_eq!(
            messages.poll_next(),
            Some(Message::canceled(Id(1), Address::default()).build())
        );
        assert_ready_eq!(messages.poll_next(), None);
    }
}
