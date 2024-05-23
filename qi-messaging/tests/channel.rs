use assert_matches::assert_matches;
use futures::{channel::mpsc, future::poll_fn, sink, stream, Sink, SinkExt, StreamExt};
use pin_project_lite::pin_project;
use qi_messaging::{
    endpoint,
    message::{Action, Address, Id, Object, OnewayRequest, Service as ServiceId},
    CapabilitiesMap, Error, Message,
};
use qi_value::{Dynamic, Value};
use std::{
    cell::RefCell,
    convert::Infallible,
    future::Future,
    marker::{PhantomData, PhantomPinned},
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{ready, Context, Poll},
};
use tokio_test::{assert_pending, assert_ready, assert_ready_err, assert_ready_ok, task};
use tower_service::Service;

#[test]
fn client_call() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (mut client, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut ready = task::spawn(poll_fn(|cx| client.poll_ready(cx)));
    assert_ready_ok!(ready.poll());

    let mut call = task::spawn(client.call((
        Address(ServiceId(1), Object(2), Action(3)),
        "My name is Alice",
    )));
    assert_pending!(call.poll());

    assert!(outgoing.is_woken());
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    let message = assert_ready!(outgoing.poll_next())
        .expect("call message is missing")
        .expect("call message is in error");
    assert_matches!(
        message,
        Message::Call {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: "My name is Alice"
        }
    );

    incoming_messages_sender
        .try_send(Ok(Message::Reply {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: "Hello Alice",
        }))
        .expect("could not send call reply");
    assert_pending!(outgoing.poll_next());

    assert!(call.is_woken());
    let reply = assert_ready_ok!(call.poll());
    assert_eq!(reply, "Hello Alice");
}

#[test]
fn client_call_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (mut client, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut ready = task::spawn(poll_fn(|cx| client.poll_ready(cx)));
    assert_ready_ok!(ready.poll());

    let mut call = task::spawn(client.call((
        Address(ServiceId(1), Object(2), Action(3)),
        "My name is Alice",
    )));
    assert_pending!(call.poll());

    assert!(outgoing.is_woken());
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_ready!(outgoing.poll_next())
        .expect("call message is missing")
        .expect("call message is in error");

    incoming_messages_sender
        .try_send(Ok(Message::Error {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            error: Dynamic("I don't know anyone named Alice".to_owned()),
        }))
        .expect("could not send call error");
    assert_pending!(outgoing.poll_next());

    assert!(call.is_woken());
    let err = assert_ready_err!(call.poll());
    assert_matches!(err, Error::Other(err) => {
        assert_eq!(err.to_string(), "I don't know anyone named Alice");
    });
}

#[test]
fn client_call_canceled() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (mut client, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut ready = task::spawn(poll_fn(|cx| client.poll_ready(cx)));
    assert_ready_ok!(ready.poll());

    let mut call = task::spawn(client.call((
        Address(ServiceId(1), Object(2), Action(3)),
        "My name is Alice",
    )));
    assert_pending!(call.poll());

    assert!(outgoing.is_woken());
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_ready!(outgoing.poll_next())
        .expect("call message is missing")
        .expect("call message is in error");

    incoming_messages_sender
        .try_send(Ok(Message::Canceled {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
        }))
        .expect("could not send call canceled");
    assert_pending!(outgoing.poll_next());

    assert!(call.is_woken());
    let err = assert_ready_err!(call.poll());
    assert_matches!(err, Error::Canceled);
}

#[test]
fn client_post() {
    let service = StrictService::new();
    let sink = StrictSink::new();
    let (mut client, outgoing) =
        endpoint(stream::empty::<Result<_, Infallible>>(), &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.send((
        Address(ServiceId(1), Object(2), Action(3)),
        OnewayRequest::Post("Say hi to Bob for me"),
    )));
    assert_ready_ok!(send.poll());

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next())
        .expect("post message is missing")
        .expect("post message is in error");
    assert_matches!(
        message,
        Message::Post {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: "Say hi to Bob for me"
        }
    )
}

#[test]
fn client_event() {
    let service = StrictService::new();
    let sink = StrictSink::new();
    let (mut client, outgoing) =
        endpoint(stream::empty::<Result<_, Infallible>>(), &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.send((
        Address(ServiceId(1), Object(2), Action(3)),
        OnewayRequest::Event("Carol says hi by the way"),
    )));
    assert_ready_ok!(send.poll());

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next())
        .expect("event message is missing")
        .expect("event message is in error");
    assert_matches!(
        message,
        Message::Event {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: "Carol says hi by the way"
        }
    )
}

#[test]
fn client_capabilities() {
    let service = StrictService::<()>::new();
    let sink = StrictSink::new();
    let (mut client, outgoing) =
        endpoint(stream::empty::<Result<_, Infallible>>(), &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.send((
        Address(ServiceId(1), Object(2), Action(3)),
        OnewayRequest::Capabilities(CapabilitiesMap::from_iter([
            ("SayHi".to_owned(), Dynamic(Value::Bool(true))),
            ("NotifyHi".to_owned(), Dynamic(Value::Int32(42))),
        ])),
    )));
    assert_ready_ok!(send.poll());

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next())
        .expect("capabilities message is missing")
        .expect("capabilities message is in error");
    assert_matches!(
        message,
        Message::Capabilities {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            capabilities
        } => {
            assert_eq!(capabilities, CapabilitiesMap::from_iter([
                ("SayHi".to_owned(), Dynamic(Value::Bool(true))),
                ("NotifyHi".to_owned(), Dynamic(Value::Int32(42))),
            ]));
        }
    )
}

#[test]
fn service_call() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(ServiceId(3), Object(2), Action(1)),
            value: "cookies",
        }))
        .expect("failed to send call message");

    assert!(outgoing.is_woken());
    service.set_state(ServiceState::NotReady);
    sink.set_state(SinkState::Ready);
    assert_pending!(outgoing.poll_next());
    assert_eq!(service.state(), ServiceState::Readying);
    // Service Sequence:
    //  - poll_ready() -> Ready, state = Ready
    //  - call(), state = NotReady
    //  - poll_ready() -> Pending, state = Readying
    // Service Future is polled once, after the dispatch, so its state is Pending = false.
    assert_pending!(outgoing.poll_next());
    assert_eq!(service.state(), ServiceState::Readying);

    let message = assert_ready!(outgoing.poll_next())
        .expect("missing reply message")
        .expect("reply message is in error");
    assert_matches!(
        message,
        Message::Reply {
            id: Id(1),
            address: Address(ServiceId(3), Object(2), Action(1)),
            value: "cookies"
        }
    );
}

#[test]
fn service_ready_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(ServiceId(3), Object(2), Action(1)),
            value: "cookies",
        }))
        .expect("failed to send call message");

    service.set_state(ServiceState::PendingError("an unknown service error"));
    sink.set_state(SinkState::Ready);
    let err = assert_ready!(outgoing.poll_next())
        .expect("missing service error")
        .unwrap_err();
    assert_matches!(err, Error::Other(err) => {
        assert_eq!(err.to_string(), "an unknown service error");
    });
}

#[test]
fn service_call_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(ServiceId(3), Object(2), Action(1)),
            value: "cookies",
        }))
        .expect("failed to send call message");

    service.set_state(ServiceState::ReadyCallError(
        "an error that will occur for the call future",
    ));
    sink.set_state(SinkState::Ready);
    // Poll next returns pending: the service is ready, it gets called and returns a pending future
    // with an error value.
    assert_pending!(outgoing.poll_next());
    // Poll next returns ready: the call future resolves and returns the error message.
    let message = assert_ready!(outgoing.poll_next())
        .expect("missing error")
        .expect("error message is in error");
    assert_matches!(
        message,
        Message::Error {
            id: Id(1),
            address: Address(ServiceId(3), Object(2), Action(1)),
            error: Dynamic(err)
        } => {
            assert_eq!(err, "an error that will occur for the call future");
        }
    )
}

/// Tests that a call to the service is correctly canceled when a cancel message is received,
/// and that the call future is consequently dropped.
#[test]
fn service_call_cancel() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = CountedPendingService::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, sink::drain());
    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address::DEFAULT,
            value: (),
        }))
        .expect("failed to send call message");

    // The service call never terminates, so there is no outgoing message yet.
    assert_pending!(outgoing.poll_next());
    assert_eq!(service.running_calls(), 1);

    // Send the cancel, then poll. The call is canceled: there is no
    // more running calls and one canceled message is produced.
    incoming_messages_sender
        .try_send(Ok(Message::Cancel {
            id: Id(2),
            address: Address::DEFAULT,
            call_id: Id(1),
        }))
        .expect("failed to send cancel message");

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next());
    assert_matches!(
        message,
        Some(Ok(Message::Canceled {
            id: Id(1),
            address: Address::DEFAULT,
        }))
    );
    assert_eq!(service.running_calls(), 0);
}

/// Tests that the service may be called multiple times without waiting for previous calls to finish.
/// This means that calls of the service can be concurrent.
#[test]
fn service_concurrent_calls() {
    // N number of service concurrent calls.
    const SERVICE_CONCURRENT_CALLS: usize = 5;

    // Send N call messages to the endpoint.
    let messages = stream::repeat(Ok::<_, Infallible>(Message::Call {
        id: Id::DEFAULT,
        address: Address::DEFAULT,
        value: (),
    }))
    .take(SERVICE_CONCURRENT_CALLS);

    let service = CountedPendingService::new();
    let (_, outgoing) = endpoint(messages, &service, sink::drain());
    let mut messages = task::spawn(outgoing);

    // Process incoming messages.
    assert_pending!(messages.poll_next());

    // Check that we have the number of expected running calls.
    assert_eq!(service.running_calls(), SERVICE_CONCURRENT_CALLS);
}

#[test]
fn sink_post() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Post {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: "Bob says hi back",
        }))
        .expect("could not send post message");

    assert!(outgoing.is_woken());
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::NotReady);
    assert_pending!(outgoing.poll_next());
    assert_eq!(sink.state(), SinkState::Readying);
    // Sink Sequence:
    //  - poll_ready() -> Ready, state = Ready
    //  - start_send() -> Ok, state = NotReady
    //  - poll_ready() -> Pending, state = Readying
    assert_pending!(outgoing.poll_next());
    assert_eq!(sink.state(), SinkState::Readying);
    assert_eq!(
        sink.data_received(),
        [(
            Address(ServiceId(1), Object(2), Action(3)),
            OnewayRequest::Post("Bob says hi back")
        )]
    );
}

#[test]
fn sink_event() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Event {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: "Carol received your 'hi'",
        }))
        .expect("could not send event message");

    assert!(outgoing.is_woken());
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_pending!(outgoing.poll_next());
    assert_eq!(
        sink.data_received(),
        [(
            Address(ServiceId(1), Object(2), Action(3)),
            OnewayRequest::Event("Carol received your 'hi'")
        )]
    );
}

#[test]
fn sink_capabilities() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let service = StrictService::<()>::new();
    let sink = StrictSink::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Capabilities {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            capabilities: CapabilitiesMap::from_iter([(
                "SayHi".to_owned(),
                Dynamic(Value::Bool(false)),
            )]),
        }))
        .expect("could not send capabilties message");

    assert!(outgoing.is_woken());
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_pending!(outgoing.poll_next());
    assert_eq!(
        sink.data_received(),
        [(
            Address(ServiceId(1), Object(2), Action(3)),
            OnewayRequest::Capabilities(CapabilitiesMap::from_iter([(
                "SayHi".to_owned(),
                Dynamic(Value::Bool(false)),
            )]))
        )]
    );
}

#[test]
fn sink_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, &'static str>>(1);

    let service = StrictService::new();
    let sink = StrictSink::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Post {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: "Bob says hi",
        }))
        .expect("could not send post message");

    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::PendingError("this is a sink error"));

    let err = assert_ready!(outgoing.poll_next())
        .expect("missing sink error")
        .unwrap_err();
    assert_matches!(err, Error::Other(err) => {
        assert_eq!(err.to_string(), "this is a sink error");
    });
}

#[test]
fn incoming_messages_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, &'static str>>(1);

    let service = StrictService::<()>::new();
    let sink = StrictSink::new();
    let (_, outgoing) = endpoint(incoming_messages_receiver, &service, &sink);

    let mut outgoing = task::spawn(outgoing);
    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Err("This is a incoming error"))
        .expect("could not send capabilties message");

    service.set_state(ServiceState::Ready);
    sink.set_state(SinkState::Ready);
    let err = assert_ready!(outgoing.poll_next())
        .expect("missing error")
        .unwrap_err();
    assert_matches!(err, Error::Other(err) => {
        assert_eq!(err.to_string(), "This is a incoming error");
    })
}

#[test]
fn endpoint_termination() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, &'static str>>(1);

    let service = CountedPendingService::new();
    let (client, outgoing) = endpoint(incoming_messages_receiver, &service, sink::drain());

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: (),
        }))
        .expect("could not send call message");

    assert_pending!(outgoing.poll_next());

    // There is one call that is running.
    assert_eq!(service.running_calls(), 1);

    // Terminating the incoming messages stream does not terminate the outgoing messages stream: there are
    // still messages that could come from either the client or the server.
    drop(incoming_messages_sender);
    assert_pending!(outgoing.poll_next());

    // Dropping the client, there could still be messages from the server.
    drop(client);

    service.unblock_futures();

    let message = assert_ready!(outgoing.poll_next())
        .expect("a reply message must be produced")
        .expect("the reply message must not be an error");
    assert_matches!(
        message,
        Message::Reply {
            id: Id(1),
            address: Address(ServiceId(1), Object(2), Action(3)),
            value: ()
        }
    );

    // Now, all the service calls are finished, client is dropped and there are not more incoming
    // messages, the stream is terminated.
    assert_matches!(assert_ready!(outgoing.poll_next()), None);
}

// A service that returns futures that block until notified and tracks how many futures are created.
struct CountedPendingService {
    unblock: Arc<tokio::sync::Notify>,
    pending_calls: Arc<AtomicUsize>,
}

impl CountedPendingService {
    fn new() -> Self {
        CountedPendingService {
            unblock: Arc::new(tokio::sync::Notify::new()),
            pending_calls: Arc::default(),
        }
    }

    fn unblock_futures(&self) {
        self.unblock.notify_waiters()
    }

    fn running_calls(&self) -> usize {
        self.pending_calls.load(Ordering::SeqCst)
    }
}

impl<'a> tower_service::Service<(Address, ())> for &'a CountedPendingService {
    type Response = ();
    type Error = Error;
    type Future = CountedPendingFuture<'a>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: (Address, ())) -> Self::Future {
        CountedPendingFuture::new(self.unblock.notified(), Arc::clone(&self.pending_calls))
    }
}

pin_project! {
    struct CountedPendingFuture<'a> {
        #[pin]
        unblocked: tokio::sync::futures::Notified<'a>,
        drop_guard: DecreaseCountDropGuard,
    }
}

impl<'a> CountedPendingFuture<'a> {
    fn new(unblocked: tokio::sync::futures::Notified<'a>, count: Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::SeqCst);
        Self {
            unblocked,
            drop_guard: DecreaseCountDropGuard(count),
        }
    }
}

impl<'a> Future for CountedPendingFuture<'a> {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        ready!(self.project().unblocked.poll(cx));
        Poll::Ready(Ok(()))
    }
}

struct DecreaseCountDropGuard(Arc<AtomicUsize>);

impl Drop for DecreaseCountDropGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StrictService<T>(RefCell<ServiceState>, PhantomData<T>);

impl<T> StrictService<T> {
    fn new() -> Self {
        let state = RefCell::new(ServiceState::NotReady);
        Self(state, PhantomData)
    }

    fn set_state(&self, state: ServiceState) {
        self.0.replace(state);
    }

    fn state(&self) -> ServiceState {
        *self.0.borrow()
    }
}

impl<T> tower_service::Service<(Address, T)> for &StrictService<T> {
    type Response = T;
    type Error = &'static str;
    type Future = StrictServiceFuture<T>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut state = self.0.borrow_mut();
        match *state {
            ServiceState::NotReady => {
                *state = ServiceState::Readying;
                Poll::Pending
            }
            ServiceState::Readying => {
                *state = ServiceState::Ready;
                Poll::Ready(Ok(()))
            }
            ServiceState::ReadyCallError(_) => Poll::Ready(Ok(())),
            ServiceState::Ready => Poll::Ready(Ok(())),
            ServiceState::PendingError(err) => {
                *state = ServiceState::Errored;
                Poll::Ready(Err(err))
            }
            ServiceState::Errored => panic!("poll_ready: service is errored"),
        }
    }

    fn call(&mut self, (_, value): (Address, T)) -> Self::Future {
        let mut state = self.0.borrow_mut();
        match *state {
            ServiceState::NotReady => panic!("call: service is not ready"),
            ServiceState::Readying => panic!("call: service is readying"),
            ServiceState::Ready => {
                *state = ServiceState::NotReady;
                StrictServiceFuture {
                    pending: true,
                    value: Some(Ok(value)),
                }
            }
            ServiceState::ReadyCallError(err) | ServiceState::PendingError(err) => {
                *state = ServiceState::NotReady;
                StrictServiceFuture {
                    pending: true,
                    value: Some(Err(err)),
                }
            }
            ServiceState::Errored => panic!("call: service is errored"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ServiceState {
    NotReady,
    Readying,
    Ready,
    ReadyCallError(&'static str),
    PendingError(&'static str),
    Errored,
}

pin_project! {
    #[project(!Unpin)]
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct StrictServiceFuture<T> {
        pending: bool,
        value: Option<Result<T, &'static str>>,
    }
}

impl<T> std::future::Future for StrictServiceFuture<T> {
    type Output = Result<T, &'static str>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.pending {
            cx.waker().wake_by_ref();
            *this.pending = false;
            Poll::Pending
        } else {
            let value = this
                .value
                .take()
                .expect("poll: polling a terminated future");
            Poll::Ready(value)
        }
    }
}

#[derive(Debug)]
struct StrictSink<T> {
    state: RefCell<(SinkState, Vec<T>)>,
    _pinned: PhantomPinned,
}

impl<T> StrictSink<T> {
    fn new() -> Self {
        let state = RefCell::new((SinkState::NotReady, Vec::new()));
        Self {
            state,
            _pinned: PhantomPinned,
        }
    }

    fn set_state(&self, state: SinkState) {
        self.state.borrow_mut().0 = state;
    }

    fn state(&self) -> SinkState {
        self.state.borrow().0
    }

    fn data_received(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.state.borrow().1.clone()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SinkState {
    NotReady,
    Readying,
    Ready,
    Flushing,
    Closing,
    Closed,
    PendingError(&'static str),
    Errored,
}

impl<T> Sink<T> for &StrictSink<T> {
    type Error = &'static str;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut state = self.state.borrow_mut();
        match state.0 {
            SinkState::NotReady => {
                state.0 = SinkState::Readying;
                Poll::Pending
            }
            SinkState::Readying => {
                state.0 = SinkState::Ready;
                Poll::Ready(Ok(()))
            }
            SinkState::Ready => Poll::Ready(Ok(())),
            SinkState::Flushing | SinkState::Closing | SinkState::Closed => Poll::Pending,
            SinkState::PendingError(err) => {
                state.0 = SinkState::Errored;
                Poll::Ready(Err(err))
            }
            SinkState::Errored => panic!("poll_ready: sink is errored"),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let mut state = self.state.borrow_mut();
        match state.0 {
            SinkState::NotReady => panic!("start_send: sink is not ready"),
            SinkState::Readying => panic!("start_send: sink is readying"),
            SinkState::Ready => {
                state.1.push(item);
                state.0 = SinkState::NotReady;
                Ok(())
            }
            SinkState::Flushing => panic!("start_send: sink is flushing"),
            SinkState::Closing => panic!("start_send: sink is closing"),
            SinkState::Closed => panic!("start_send: sink is closed"),
            SinkState::PendingError(err) => {
                state.0 = SinkState::Errored;
                Err(err)
            }
            SinkState::Errored => panic!("start_send: sink is errored"),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut state = self.state.borrow_mut();
        match state.0 {
            SinkState::NotReady | SinkState::Readying | SinkState::Ready => {
                state.0 = SinkState::Flushing;
                Poll::Pending
            }
            SinkState::Flushing => {
                state.0 = SinkState::NotReady;
                Poll::Ready(Ok(()))
            }
            SinkState::Closing => panic!("poll_flush: sink is closing"),
            SinkState::Closed => panic!("poll_flush: sink is closed"),
            SinkState::PendingError(err) => {
                state.0 = SinkState::Errored;
                Poll::Ready(Err(err))
            }
            SinkState::Errored => panic!("poll_flush: sink is errored"),
        }
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut state = self.state.borrow_mut();
        match state.0 {
            SinkState::NotReady | SinkState::Readying | SinkState::Ready => {
                state.0 = SinkState::Closing;
                Poll::Pending
            }
            SinkState::Flushing => {
                state.0 = SinkState::Closing;
                Poll::Pending
            }
            SinkState::Closing => {
                state.0 = SinkState::Closed;
                Poll::Ready(Ok(()))
            }
            SinkState::Closed => panic!("poll_close: sink is closed"),
            SinkState::PendingError(err) => {
                state.0 = SinkState::Errored;
                Poll::Ready(Err(err))
            }
            SinkState::Errored => panic!("poll_close: sink is errored"),
        }
    }
}
