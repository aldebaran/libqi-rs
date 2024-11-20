use assert_matches::assert_matches;
use futures::{channel::mpsc, stream, StreamExt};
use qi_messaging::{
    endpoint,
    message::{self, Action, Address, FireAndForget, Id, Object, Service},
    Error, Handler, Message,
};
use qi_value::{KeyDynValueMap, Value};
use std::{
    convert::Infallible,
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio_test::{
    assert_pending, assert_ready, assert_ready_eq, assert_ready_err, assert_ready_ok, task,
};

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
        Ok("My name is Alice"),
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
            value,
        } => {
            assert_eq!(value, Ok("My name is Alice"));
        }
    );

    incoming_messages_sender
        .try_send(Ok(Message::Reply {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            value: Ok("Hello Alice (from server)"),
        }))
        .expect("could not send call reply");
    assert_pending!(outgoing.poll_next());

    assert!(call.is_woken());
    let reply = assert_ready_ok!(call.poll());
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
        Ok("My name is Alice"),
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
        Ok("My name is Alice"),
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

    let mut send = task::spawn(client.fire_and_forget(
        Address(Service(1), Object(2), Action(3)),
        FireAndForget::Post(Ok("Say hi to Bob for me")),
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
            value
        } => {
            assert_eq!(value, Ok("Say hi to Bob for me"));
        }
    )
}

#[test]
fn client_event() {
    let (handler, _) = SimpleHandler::new();
    let (client, outgoing) = endpoint::dispatch(stream::empty::<Result<_, Infallible>>(), handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.fire_and_forget(
        Address(Service(1), Object(2), Action(3)),
        FireAndForget::Event(Ok("Carol says hi by the way")),
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
            value
        } => {
            assert_eq!(value, Ok("Carol says hi by the way"));
        }
    )
}

#[test]
fn client_capabilities() {
    let (handler, _) = SimpleHandler::new();
    let (client, outgoing) = endpoint::dispatch(stream::empty::<Result<_, Infallible>>(), handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.fire_and_forget(
        Address(Service(1), Object(2), Action(3)),
        FireAndForget::Capabilities(KeyDynValueMap::from_iter([
            ("SayHi".to_owned(), Value::Bool(true)),
            ("NotifyHi".to_owned(), Value::Int32(42)),
        ])),
    ));
    assert_ready_ok!(send.poll());

    assert!(outgoing.is_woken());
    let message = assert_ready!(outgoing.poll_next())
        .expect("capabilities message is missing")
        .expect("capabilities message is in error");
    assert_matches!(
        message,
        Message::Capabilities {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            capabilities
        } => {
            assert_eq!(capabilities, KeyDynValueMap::from_iter([
                ("SayHi".to_owned(), Value::Bool(true)),
                ("NotifyHi".to_owned(), Value::Int32(42)),
            ]));
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
            value: Ok("My name is Alice"),
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
            value
        } => {
            assert_eq!(value, Ok("My name is Alice"));
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
            value: Err(HandlerError {
                message: "bad request",
                is_canceled: false,
                is_fatal: false,
            }),
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
            value: Err(HandlerError {
                message: "fatal request",
                is_canceled: false,
                is_fatal: true,
            }),
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
            value: Err(HandlerError {
                message: "canceled",
                is_canceled: true,
                is_fatal: false,
            }),
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
            address: Address::DEFAULT,
            value: (),
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
        id: Id::DEFAULT,
        address: Address::DEFAULT,
        value: (),
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

    let (handler, faf_receiver) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut faf = task::spawn(faf_receiver);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Post {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            value: Ok("Bob says hi back"),
        }))
        .expect("could not send post message");

    assert!(outgoing.is_woken());
    assert_pending!(outgoing.poll_next());
    assert_ready_eq!(
        faf.poll_next(),
        Some((
            Address(Service(1), Object(2), Action(3)),
            FireAndForget::Post(Ok("Bob says hi back"))
        ))
    );
}

#[test]
fn handler_event() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, faf_receiver) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut faf = task::spawn(faf_receiver);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Event {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            value: Ok("Carol received your 'hi'"),
        }))
        .expect("could not send event message");

    assert!(outgoing.is_woken());
    assert_pending!(outgoing.poll_next());
    assert_ready_eq!(
        faf.poll_next(),
        Some((
            Address(Service(1), Object(2), Action(3)),
            FireAndForget::Event(Ok("Carol received your 'hi'"))
        ))
    );
}

#[test]
fn handler_capabilities() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, faf_receiver) = SimpleHandler::new();
    let (_client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut faf = task::spawn(faf_receiver);

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
    assert_ready_eq!(
        faf.poll_next(),
        Some((
            Address(Service(1), Object(2), Action(3)),
            FireAndForget::Capabilities(KeyDynValueMap::from_iter([(
                "SayHi".to_owned(),
                Value::Bool(false),
            )]))
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

#[test]
fn endpoint_termination() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, std::convert::Infallible>>(1);

    let handler = CountedPendingHandler::new();
    let (client, outgoing) = endpoint::dispatch(incoming_messages_receiver, &handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            value: (),
        }))
        .expect("could not send call message");

    assert_pending!(outgoing.poll_next());

    // There is one call that is running.
    assert_eq!(handler.running_calls(), 1);

    // Terminating the incoming messages stream does not terminate the outgoing messages stream: there are
    // still messages that could come from either the client or the server.
    drop(incoming_messages_sender);
    assert_pending!(outgoing.poll_next());

    // Dropping the client, there could still be messages from the server.
    drop(client);

    handler.unblock_futures();

    let message = assert_ready!(outgoing.poll_next())
        .expect("a reply message must be produced")
        .expect("the reply message must not be an error");
    assert_matches!(
        message,
        Message::Reply {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            value: ()
        }
    );

    // Now, all the handler calls are finished, client is dropped and there are not more incoming
    // messages, the stream is terminated.
    assert_matches!(assert_ready!(outgoing.poll_next()), None);
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, PartialOrd, Ord)]
#[error("{message}")]
struct HandlerError {
    message: &'static str,
    is_canceled: bool,
    is_fatal: bool,
}

impl qi_messaging::handler::Error for HandlerError {
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

    fn unblock_futures(&self) {
        self.unblock.notify_waiters()
    }

    fn running_calls(&self) -> usize {
        self.pending_calls.load(Ordering::SeqCst)
    }
}

impl<'a> Handler<()> for &'a CountedPendingHandler {
    type Error = Infallible;

    fn call(
        &self,
        _address: message::Address,
        _: (),
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let drop_guard = DecreaseCountDropGuard::new(&self.pending_calls);
        let unblock = Arc::clone(&self.unblock);
        async move {
            unblock.notified().await;
            drop(drop_guard);
            Ok(())
        }
    }

    async fn fire_and_forget(
        &self,
        _address: message::Address,
        _request: message::FireAndForget<()>,
    ) {
    }
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
struct SimpleHandler(mpsc::UnboundedSender<(message::Address, FireAndForget<HandlerValue>)>);

impl SimpleHandler {
    fn new() -> (
        Self,
        mpsc::UnboundedReceiver<(message::Address, FireAndForget<HandlerValue>)>,
    ) {
        let (sender, receiver) = mpsc::unbounded();
        (Self(sender), receiver)
    }
}

impl Handler<HandlerValue> for SimpleHandler {
    type Error = HandlerError;

    async fn call(
        &self,
        _address: message::Address,
        value: HandlerValue,
    ) -> Result<Result<&'static str, HandlerError>, HandlerError> {
        value.map(Ok)
    }

    async fn fire_and_forget(
        &self,
        address: message::Address,
        request: message::FireAndForget<HandlerValue>,
    ) {
        self.0.unbounded_send((address, request)).unwrap()
    }
}

type HandlerValue = Result<&'static str, HandlerError>;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct StreamError(&'static str);
