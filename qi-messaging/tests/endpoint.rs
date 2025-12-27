use assert_matches::assert_matches;
use bytes::Bytes;
use futures::{
    channel::mpsc,
    future::{err, ok, BoxFuture, Ready},
    stream, FutureExt, StreamExt,
};
use qi_format::{from_slice, to_bytes};
use qi_messaging::{
    endpoint,
    handler::CallError,
    message::{self, Action, Address, Id, Object, Service},
    CallHandler, CapabilitiesHandler, Error, EventHandler, Message, PostHandler,
};
use qi_value::{KeyDynValueMap, Value};
use std::{
    convert::Infallible,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio_test::{assert_pending, assert_ready, assert_ready_err, assert_ready_ok, task};

#[test]

fn client_call() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = SimpleHandler::new();

    let (client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut call = task::spawn(client.call(
        Address(Service(1), Object(2), Action(3)),
        to_bytes(&HandlerValue::Ok("My name is Alice")).unwrap(),
    ));
    assert_pending!(call.poll());

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next())
        .expect("call message is missing")
        .expect("call message is in error");
    assert_matches!(
        message,
        Message::Call {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            payload,
        } => {
            let value = from_slice::<HandlerValue>(&payload).unwrap();
            assert_eq!(value, Ok("My name is Alice"));
        }
    );

    incoming_messages_sender
        .try_send(Ok(Message::Reply {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            payload: to_bytes(&HandlerValue::Ok("Hello Alice (from server)")).unwrap(),
        }))
        .expect("could not send call reply");
    assert_pending!(outgoing.poll_next());

    assert!(call.is_woken());
    let reply = assert_ready_ok!(call.poll());
    let reply = from_slice::<HandlerValue>(&reply).unwrap();
    assert_eq!(reply, Ok("Hello Alice (from server)"));
}

#[test]
fn client_call_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = SimpleHandler::new();
    let (client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut call = task::spawn(client.call(
        Address(Service(1), Object(2), Action(3)),
        Bytes::from_static(b"My name is Alice"),
    ));
    assert_pending!(call.poll());

    assert!(outgoing.is_woken());
    assert_ready!(outgoing.poll_next())
        .expect("call message is missing")
        .expect("call message is in error");

    incoming_messages_sender
        .try_send(Ok(Message::Error {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            error: "I don't know anyone named Alice".to_owned(),
        }))
        .expect("could not send call error");
    assert_pending!(outgoing.poll_next());

    assert!(call.is_woken());
    let err = assert_ready_err!(call.poll());
    assert_matches!(err, Error::CallError(err) => {
        assert_eq!(err, "I don't know anyone named Alice");
    });
}

#[test]
fn client_call_canceled() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = SimpleHandler::new();
    let (client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut call = task::spawn(client.call(
        Address(Service(1), Object(2), Action(3)),
        Bytes::from_static(b"My name is Alice"),
    ));
    assert_pending!(call.poll());

    assert!(outgoing.is_woken());
    assert_ready!(outgoing.poll_next())
        .expect("call message is missing")
        .expect("call message is in error");

    incoming_messages_sender
        .try_send(Ok(Message::Canceled {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
        }))
        .expect("could not send call canceled");
    assert_pending!(outgoing.poll_next());

    assert!(call.is_woken());
    let err = assert_ready_err!(call.poll());
    assert_matches!(err, Error::CallCanceled);
}

#[test]
fn client_post() {
    let (handler, _) = SimpleHandler::new();
    let (client, outgoing) = endpoint::dispatch(stream::empty::<Result<_, Infallible>>(), handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.post(
        Address(Service(1), Object(2), Action(3)),
        Bytes::from_static(b"Say hi to Bob for me"),
    ));
    assert_ready_ok!(send.poll());

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next())
        .expect("post message is missing")
        .expect("post message is in error");
    assert_matches!(
        message,
        Message::Post {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            payload
        } => {
            assert_eq!(payload, b"Say hi to Bob for me".as_slice());
        }
    )
}

#[test]
fn client_event() {
    let (handler, _) = SimpleHandler::new();
    let (client, outgoing) = endpoint::dispatch(stream::empty::<Result<_, Infallible>>(), handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.send_event(
        Address(Service(1), Object(2), Action(3)),
        Bytes::from_static(b"Carol says hi by the way"),
    ));
    assert_ready_ok!(send.poll());

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next())
        .expect("event message is missing")
        .expect("event message is in error");
    assert_matches!(
        message,
        Message::Event {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            payload
        } => {
            assert_eq!(payload, b"Carol says hi by the way".as_slice());
        }
    )
}

#[test]
fn client_drop_closes_endpoint() {
    let (handler, _) = SimpleHandler::new();
    let (client, outgoing) = endpoint::dispatch(stream::empty::<Result<_, Infallible>>(), handler);
    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());
    drop(client);
    assert_matches!(assert_ready!(outgoing.poll_next()), None);
}

#[test]
fn handler_call() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            payload: to_bytes(&HandlerValue::Ok("My name is Alice")).unwrap(),
        }))
        .expect("failed to send call message");

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next())
        .expect("missing reply message")
        .expect("reply message is in error");
    assert_matches!(
        message,
        Message::Reply {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            payload
        } => {
            let value = from_slice::<&str>(&payload).unwrap();
            assert_eq!(value, "My name is Alice");
        }
    );
}

#[test]
fn handler_call_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            payload: to_bytes(&Err::<&str, _>(HandlerError {
                message: "bad request".to_owned(),
                is_canceled: false,
                is_fatal: false,
            }))
            .unwrap(),
        }))
        .expect("failed to send call message");

    let message = assert_ready!(outgoing.poll_next())
        .expect("missing message error")
        .expect("error message is not ok");
    assert_matches!(
        message,
        Message::Error {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            error
        } => {
            assert_eq!(error.to_string(), "bad request");
        }
    );
}

#[test]
fn handler_call_error_fatal() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            payload: to_bytes(&Err::<&str, _>(HandlerError {
                message: "fatal request".to_owned(),
                is_canceled: false,
                is_fatal: true,
            }))
            .unwrap(),
        }))
        .expect("failed to send call message");

    let message = assert_ready!(outgoing.poll_next())
        .expect("missing message error")
        .expect("error message is not ok");
    // Dispatch still sends the error back to the caller before stopping.
    assert_matches!(
        message,
        Message::Error {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            error
        } => {
            assert_eq!(error.to_string(), "fatal request");
        }
    );

    // Error is fatal, dispatch is ended.
    assert_matches!(assert_ready!(outgoing.poll_next()), None);
}

#[test]
fn handler_call_canceled() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            payload: to_bytes(&HandlerValue::Err(HandlerError {
                message: "canceled".to_owned(),
                is_canceled: true,
                is_fatal: false,
            }))
            .unwrap(),
        }))
        .expect("failed to send call message");

    let message = assert_ready!(outgoing.poll_next())
        .expect("missing message canceled")
        .expect("canceled message is not ok");
    // Dispatch still sends the error back to the caller before stopping.
    assert_matches!(
        message,
        Message::Canceled {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
        }
    );
}

/// Tests that a call to the handler is correctly canceled when a cancel message is received,
/// and that the call future is consequently dropped.
#[test]
fn handler_call_cancel() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let handler = CountedPendingHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, &handler);
    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address::default(),
            payload: Bytes::new(),
        }))
        .expect("failed to send call message");

    // The handler call never terminates, so there is no outgoing message yet.
    assert_pending!(outgoing.poll_next());
    assert_eq!(handler.running_calls(), 1);

    // Send the cancel, then poll. The call is canceled: there is no
    // more running calls and one canceled message is produced.
    incoming_messages_sender
        .try_send(Ok(Message::Cancel {
            id: Id(2),
            address: Address::default(),
            call_id: Id(1),
        }))
        .expect("failed to send cancel message");

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next());
    assert_matches!(
        message,
        Some(Ok(Message::Canceled {
            id: Id(1),
            address
        })) if address == Address::default()
    );
    assert_eq!(handler.running_calls(), 0);
}

/// Tests that the handler may be called multiple times without waiting for previous calls to finish.
/// This means that calls of the handler can be concurrent.
#[test]
fn handler_concurrent_calls() {
    // N number of handler concurrent calls.
    const HANDLER_CONCURRENT_CALLS: usize = 5;

    // Send N call messages to the endpoint.
    let messages = stream::repeat(Ok::<_, Infallible>(Message::Call {
        id: Id::default(),
        address: Address::default(),
        payload: Bytes::new(),
    }))
    .take(HANDLER_CONCURRENT_CALLS);

    let handler = CountedPendingHandler::new();
    let (_client, outgoing) = endpoint::dispatch(messages, &handler);
    let mut messages = task::spawn(outgoing);

    // Process incoming messages.
    assert_pending!(messages.poll_next());

    // Check that we have the number of expected running calls.
    assert_eq!(handler.running_calls(), HANDLER_CONCURRENT_CALLS);
}

#[test]
fn handler_post() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, SimpleHandlerReceivers { posts, .. }) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut posts = task::spawn(posts);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Post {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            payload: Bytes::from_static(b"Bob says hi back"),
        }))
        .expect("could not send post message");

    assert!(outgoing.is_woken());
    assert_pending!(outgoing.poll_next());
    let post = assert_ready!(posts.poll_next());
    assert_matches!(
        post,
        Some((Address(Service(1), Object(2), Action(3)), args)) => {
            assert_eq!(args, Bytes::from_static(b"Bob says hi back"));
        }
    );
}

#[test]
fn handler_event() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, SimpleHandlerReceivers { events, .. }) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut events = task::spawn(events);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Event {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            payload: Bytes::from_static(b"Carol received your 'hi'"),
        }))
        .expect("could not send event message");

    assert!(outgoing.is_woken());
    assert_pending!(outgoing.poll_next());
    let event = assert_ready!(events.poll_next());
    assert_matches!(
        event,
        Some((
            Address(Service(1), Object(2), Action(3)),
            args,
        )) => {
            assert_eq!(args, b"Carol received your 'hi'".as_slice());
        }
    );
}

#[test]
fn handler_capabilities() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, SimpleHandlerReceivers { capabilities, .. }) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut capabilities = task::spawn(capabilities);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Capabilities {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            capabilities: KeyDynValueMap::from_iter([("SayHi".to_owned(), Value::Bool(false))]),
        }))
        .expect("could not send capabilties message");

    assert!(outgoing.is_woken());
    assert_pending!(outgoing.poll_next());
    let capabilities = assert_ready!(capabilities.poll_next());
    assert_eq!(
        capabilities,
        Some((
            Address(Service(1), Object(2), Action(3)),
            KeyDynValueMap::from_iter([("SayHi".to_owned(), Value::Bool(false),)])
        ))
    );
}

#[test]
fn incoming_messages_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, StreamError>>(1);

    let (handler, _) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Err(StreamError("This is a incoming error")))
        .expect("could not send capabilties message");

    let StreamError(err) = assert_ready!(outgoing.poll_next())
        .expect("missing error")
        .unwrap_err();
    assert_eq!(err, "This is a incoming error");
}

#[derive(
    Debug, thiserror::Error, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[error("{message}")]
struct HandlerError {
    message: String,
    is_canceled: bool,
    is_fatal: bool,
}

impl CallError for HandlerError {
    fn is_canceled(&self) -> bool {
        self.is_canceled
    }

    fn is_fatal(&self) -> bool {
        self.is_fatal
    }
}

// A handler that returns futures that block until notified and tracks how many futures are created.
struct CountedPendingHandler {
    unblock: Arc<tokio::sync::Notify>,
    pending_calls: Arc<AtomicUsize>,
}

impl CountedPendingHandler {
    fn new() -> Self {
        CountedPendingHandler {
            unblock: Arc::new(tokio::sync::Notify::new()),
            pending_calls: Arc::default(),
        }
    }

    fn running_calls(&self) -> usize {
        self.pending_calls.load(Ordering::SeqCst)
    }
}

impl CallHandler for &'_ CountedPendingHandler {
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Bytes, Self::Error>>;

    fn handle_call(&self, _address: message::Address, _: Bytes) -> Self::Future {
        let drop_guard = DecreaseCountDropGuard::new(&self.pending_calls);
        let unblock = Arc::clone(&self.unblock);
        async move {
            unblock.notified().await;
            drop(drop_guard);
            Ok(Bytes::new())
        }
        .boxed()
    }
}

impl EventHandler for &'_ CountedPendingHandler {
    fn handle_event(&self, _address: message::Address, _args: Bytes) {}
}

impl PostHandler for &'_ CountedPendingHandler {
    fn handle_post(&self, _address: message::Address, _args: Bytes) {}
}

impl CapabilitiesHandler for &'_ CountedPendingHandler {
    fn handle_capabilities(&self, _address: message::Address, _data: KeyDynValueMap) {}
}

struct DecreaseCountDropGuard(Arc<AtomicUsize>);

impl DecreaseCountDropGuard {
    fn new(count: &Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::SeqCst);
        Self(Arc::clone(count))
    }
}

impl Drop for DecreaseCountDropGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone)]
struct SimpleHandler {
    events: mpsc::UnboundedSender<(message::Address, Bytes)>,
    posts: mpsc::UnboundedSender<(message::Address, Bytes)>,
    capabilities: mpsc::UnboundedSender<(message::Address, KeyDynValueMap)>,
}

impl SimpleHandler {
    fn new() -> (Self, SimpleHandlerReceivers) {
        let (events_sender, events_receiver) = mpsc::unbounded();
        let (posts_sender, posts_receiver) = mpsc::unbounded();
        let (capabilities_sender, capabilities_receiver) = mpsc::unbounded();
        (
            Self {
                events: events_sender,
                posts: posts_sender,
                capabilities: capabilities_sender,
            },
            SimpleHandlerReceivers {
                events: events_receiver,
                posts: posts_receiver,
                capabilities: capabilities_receiver,
            },
        )
    }
}

impl CallHandler for SimpleHandler {
    type Error = HandlerError;
    type Future = Ready<Result<Bytes, Self::Error>>;

    fn handle_call(&self, _address: message::Address, args: Bytes) -> Self::Future {
        let arg = from_slice::<HandlerValue>(&args).unwrap();
        match arg {
            Ok(arg) => ok(to_bytes(&arg).unwrap()),
            Err(error) => err(error),
        }
    }
}

impl EventHandler for SimpleHandler {
    fn handle_event(&self, address: message::Address, args: Bytes) {
        self.events.unbounded_send((address, args)).unwrap()
    }
}

impl PostHandler for SimpleHandler {
    fn handle_post(&self, address: message::Address, args: Bytes) {
        self.posts.unbounded_send((address, args)).unwrap()
    }
}

impl CapabilitiesHandler for SimpleHandler {
    fn handle_capabilities(&self, address: message::Address, map: KeyDynValueMap) {
        self.capabilities.unbounded_send((address, map)).unwrap()
    }
}

#[derive(Debug)]
struct SimpleHandlerReceivers {
    events: mpsc::UnboundedReceiver<(message::Address, Bytes)>,
    posts: mpsc::UnboundedReceiver<(message::Address, Bytes)>,
    capabilities: mpsc::UnboundedReceiver<(message::Address, KeyDynValueMap)>,
}

type HandlerValue<'a> = Result<&'a str, HandlerError>;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct StreamError(&'static str);
