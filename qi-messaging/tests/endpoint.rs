use assert_matches::assert_matches;
use futures::{channel::mpsc, future::BoxFuture, stream, FutureExt, StreamExt};
use pin_project_lite::pin_project;
use qi_messaging::{
    endpoint,
    message::{self, Action, Address, Id, Object, OnewayRequest, Service},
    CapabilitiesMap, Error, Handler, Message,
};
use qi_value::{Dynamic, Value};
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{ready, Context, Poll},
};
use tokio_test::{
    assert_pending, assert_ready, assert_ready_eq, assert_ready_err, assert_ready_ok, task,
};

#[test]
fn client_call() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = DummyHandler::new();
    let (client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut call = task::spawn(client.call(
        Address(Service(1), Object(2), Action(3)),
        "My name is Alice".to_string(),
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
            assert_eq!(value, "My name is Alice");
        }
    );

    incoming_messages_sender
        .try_send(Ok(Message::Reply {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            value: "Hello Alice (from server)".to_string(),
        }))
        .expect("could not send call reply");
    assert_pending!(outgoing.poll_next());

    assert!(call.is_woken());
    let reply = assert_ready_ok!(call.poll());
    assert_eq!(reply, "Hello Alice (from server)");
}

#[test]
fn client_call_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = DummyHandler::new();
    let (client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut call = task::spawn(client.call(
        Address(Service(1), Object(2), Action(3)),
        "My name is Alice".to_string(),
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

    let (handler, _) = DummyHandler::new();
    let (client, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut call = task::spawn(client.call(
        Address(Service(1), Object(2), Action(3)),
        "My name is Alice".to_string(),
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
    assert_matches!(err, Error::Canceled);
}

#[test]
fn client_post() {
    let (handler, _) = DummyHandler::new();
    let (client, outgoing) = endpoint::dispatch(stream::empty::<Result<_, Infallible>>(), handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.oneway(
        Address(Service(1), Object(2), Action(3)),
        OnewayRequest::Post("Say hi to Bob for me".to_string()),
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
            assert_eq!(value, "Say hi to Bob for me");
        }
    )
}

#[test]
fn client_event() {
    let (handler, _) = DummyHandler::new();
    let (client, outgoing) = endpoint::dispatch(stream::empty::<Result<_, Infallible>>(), handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.oneway(
        Address(Service(1), Object(2), Action(3)),
        OnewayRequest::Event("Carol says hi by the way".to_string()),
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
            assert_eq!(value, "Carol says hi by the way");
        }
    )
}

#[test]
fn client_capabilities() {
    let (handler, _) = DummyHandler::new();
    let (client, outgoing) = endpoint::dispatch(stream::empty::<Result<_, Infallible>>(), handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    let mut send = task::spawn(client.oneway(
        Address(Service(1), Object(2), Action(3)),
        OnewayRequest::Capabilities(CapabilitiesMap::from_iter([
            ("SayHi".to_owned(), Dynamic(Value::Bool(true))),
            ("NotifyHi".to_owned(), Dynamic(Value::Int32(42))),
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
            assert_eq!(capabilities, CapabilitiesMap::from_iter([
                ("SayHi".to_owned(), Dynamic(Value::Bool(true))),
                ("NotifyHi".to_owned(), Dynamic(Value::Int32(42))),
            ]));
        }
    )
}

#[test]
fn handler_call() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = DummyHandler::new();
    let (_, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            value: "My name is Alice".to_string(),
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
            assert_eq!(value, "Hello Alice");
        }
    );
}

#[test]
fn handler_call_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, _) = DummyHandler::new();
    let (_, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);

    incoming_messages_sender
        .try_send(Ok(Message::Call {
            id: Id(1),
            address: Address(Service(3), Object(2), Action(1)),
            value: "cookies".to_string(),
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
            error: Dynamic(error)
        } => {
            assert_eq!(error.to_string(), "not a hello request");
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
    let (_, outgoing) = endpoint::dispatch(incoming_messages_receiver, &handler);
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
    let (_, outgoing) = endpoint::dispatch(messages, &handler);
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

    let (handler, oneway_receiver) = DummyHandler::new();
    let (_, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut oneway = task::spawn(oneway_receiver);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Post {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            value: "Bob says hi back".to_string(),
        }))
        .expect("could not send post message");

    assert!(outgoing.is_woken());
    assert_pending!(outgoing.poll_next());
    assert_ready_eq!(
        oneway.poll_next(),
        Some((
            Address(Service(1), Object(2), Action(3)),
            OnewayRequest::Post("Bob says hi back".to_string())
        ))
    );
}

#[test]
fn handler_event() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, oneway_receiver) = DummyHandler::new();
    let (_, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut oneway = task::spawn(oneway_receiver);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Event {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            value: "Carol received your 'hi'".to_string(),
        }))
        .expect("could not send event message");

    assert!(outgoing.is_woken());
    assert_pending!(outgoing.poll_next());
    assert_ready_eq!(
        oneway.poll_next(),
        Some((
            Address(Service(1), Object(2), Action(3)),
            OnewayRequest::Event("Carol received your 'hi'".to_string())
        ))
    );
}

#[test]
fn handler_capabilities() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, Infallible>>(1);

    let (handler, oneway_receiver) = DummyHandler::new();
    let (_, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);
    let mut oneway = task::spawn(oneway_receiver);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Ok(Message::Capabilities {
            id: Id(1),
            address: Address(Service(1), Object(2), Action(3)),
            capabilities: CapabilitiesMap::from_iter([(
                "SayHi".to_owned(),
                Dynamic(Value::Bool(false)),
            )]),
        }))
        .expect("could not send capabilties message");

    assert!(outgoing.is_woken());
    assert_pending!(outgoing.poll_next());
    assert_ready_eq!(
        oneway.poll_next(),
        Some((
            Address(Service(1), Object(2), Action(3)),
            OnewayRequest::Capabilities(CapabilitiesMap::from_iter([(
                "SayHi".to_owned(),
                Dynamic(Value::Bool(false)),
            )]))
        ))
    );
}

#[test]
fn incoming_messages_error() {
    let (mut incoming_messages_sender, incoming_messages_receiver) =
        mpsc::channel::<Result<_, StrError>>(1);

    let (handler, _) = DummyHandler::new();
    let (_, outgoing) = endpoint::dispatch(incoming_messages_receiver, handler);

    let mut outgoing = task::spawn(outgoing);
    assert_pending!(outgoing.poll_next());

    incoming_messages_sender
        .try_send(Err(StrError("This is a incoming error")))
        .expect("could not send capabilties message");

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

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct StrError(&'static str);

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
    type Reply = ();
    type Error = Error;
    type Future = CountedPendingFuture<'a>;

    fn call(&mut self, _address: message::Address, _: ()) -> Self::Future {
        CountedPendingFuture::new(self.unblock.notified(), Arc::clone(&self.pending_calls))
    }

    fn oneway_request(
        &mut self,
        _address: message::Address,
        _request: message::OnewayRequest<()>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
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

#[derive(Debug, Clone)]
struct DummyHandler(mpsc::UnboundedSender<(message::Address, OnewayRequest<String>)>);

impl DummyHandler {
    fn new() -> (
        Self,
        mpsc::UnboundedReceiver<(message::Address, OnewayRequest<String>)>,
    ) {
        let (sender, receiver) = mpsc::unbounded();
        (Self(sender), receiver)
    }
}

impl Handler<String> for DummyHandler {
    type Error = Error;
    type Reply = String;
    type Future = BoxFuture<'static, Result<String, Error>>;

    fn call(&mut self, _address: message::Address, value: String) -> Self::Future {
        async move {
            let name = value
                .strip_prefix("My name is ")
                .ok_or(Error::other("not a hello request"))?;
            Ok(format!("Hello {name}"))
        }
        .boxed()
    }

    fn oneway_request(
        &mut self,
        address: message::Address,
        request: message::OnewayRequest<String>,
    ) -> Result<(), Self::Error> {
        self.0
            .unbounded_send((address, request))
            .map_err(Error::other)
    }
}
