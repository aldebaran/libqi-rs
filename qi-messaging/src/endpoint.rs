use crate::{
    client,
    message::Response,
    server::{self, DispatchFlow},
    Client, Handler, Message,
};
use async_stream::stream;
use either::Either;
use futures::{
    pin_mut, stream::FusedStream, Sink, SinkExt, Stream, StreamExt, TryStream, TryStreamExt,
};
use std::future::Future;
use tokio::select;

pub fn start<MsgStream, MsgSink, Handler>(
    messages_stream: MsgStream,
    messages_sink: MsgSink,
    handler: Handler,
) -> (
    Client,
    impl Future<Output = DispatchResult<MsgStream::Error, MsgSink::Error>>,
)
where
    MsgStream: TryStream<Ok = Message>,
    MsgSink: Sink<Message>,
    Handler: crate::Handler,
{
    let (client, outgoing_messages) = dispatch(messages_stream, handler);
    let connection = outgoing_messages
        .map_err(Either::Left)
        .forward(messages_sink.sink_map_err(Either::Right));
    (client, connection)
}

pub type DispatchResult<E1, E2> = Result<(), Either<E1, E2>>;

/// Returns a stream of outgoing messages of an endpoint.
///
/// It selects between two streams:
///   - incoming messages,
///   - client requests,
///
/// Client requests are the source of outgoing messages of types Call, Post, Event, Capabilities and
/// Cancel. They originate only from clients objects request channel but never from the incoming
/// messages.
///
/// Server responses are the source of outgoing messages of types Reply, Error and
/// Canceled. They originate from a sequencing between incoming messages and handler calls.
///
/// Incoming messages also have side-effects on the results sent to clients.
///
/// ```text
///                ┌───────────────────────┐
///                │                       │
///                │   Incoming Messages   │
///                │                       │
///                └───────────┬───────────┘
///                            │
///    ┌─────┬────┬─────┬──────┴──┬────────┬──────┬───────┐
///    │     │    │     │         │        │      │       │
///  Call Cancel Post Event Capabilities Reply Canceled Error
///    │     │    │     │         │        │      │       │              ┌─────────┐
/// ┌──▼─────▼────▼─────▼─────────▼─────┬──▼──────▼───────▼──┐           │         ├┐
/// │                                   │                    │           │ Clients ││
/// │              Request              │      Response      │           │         ││
/// │                                   │                    │           └┬────────┘│
/// └──┬─────┬────┬─────┬─────────┬─────┴──┬──────┬───────┬──┘            └─────────┘
///    │     │    │     │         │        │      │       │     Call Cancel Post Event Capababilities
///  ┌─▼─────▼─┬──▼─────▼─────────▼────┐ ┌─▼──────▼───────▼─┐     │    │     │     │         │
///  │         │                       │ │                  │     │    │     │     │         │
///  │ Handler │Event/Post/Capabilities│ │      Client      ◄─────┴────┴─────┴─────┴─────────┘
///  │  Calls  │        Handler        │ │     Requests     │
///  │         │                       │ │                  │
///  └────┬────┴───────────────────────┘ └────────┬─────────┘
///    Server                                     │
///   Responses                                   │
///       └───────────────┐      ┌────────────────┘
///                       │      │
///               ┌───────▼──────▼────────┐
///               │                       │
///               │   Outgoing Messages   │
///               │                       │
///               └───────────────────────┘
///```
pub fn dispatch<MsgStream, H>(
    messages: MsgStream,
    handler: H,
) -> (
    Client,
    impl Stream<Item = Result<Message, MsgStream::Error>>,
)
where
    MsgStream: TryStream<Ok = Message>,
    H: Handler,
{
    let messages = messages.into_stream().fuse();
    let (client, client_requests) = client::new_with_requests(32);
    let mut dispatch = Dispatch::new(handler, client_requests);
    let outgoing_messages = stream! {
        pin_mut!(messages);
        loop {
            select! {
                Some(message) = messages.next(), if !messages.is_terminated() => {
                    match message {
                        Ok(message) => dispatch.dispatch_message(message),
                        Err(err) => {
                            yield Err(err);
                            break
                        }
                    }
                }
                message = dispatch.client_requests.next() => {
                    match message {
                        Some(message) => yield Ok(message),
                        None => break,
                    }
                }
                Some((message, flow)) = dispatch.server_calls.next(), if !dispatch.server_calls.is_terminated() => {
                    yield Ok(message);
                    if let DispatchFlow::Stop = flow {
                        break;
                    }
                }
                else => {
                    break
                }
            }
        }
    };
    (client, outgoing_messages)
}

#[derive(Debug)]
struct Dispatch<H, E> {
    handler: H,
    client_requests: client::Requests,
    server_calls: server::CallFutures<E>,
}

impl<H> Dispatch<H, H::Error>
where
    H: Handler,
{
    fn new(handler: H, client_requests: client::Requests) -> Self {
        Self {
            handler,
            client_requests,
            server_calls: server::CallFutures::default(),
        }
    }

    fn dispatch_message(&mut self, message: Message) {
        match message {
            Message::Call {
                id,
                address,
                payload,
            } => {
                let call_future = self.handler.handle_call(address, payload);
                self.server_calls.push(id, address, call_future);
            }
            Message::Post {
                address, payload, ..
            } => {
                self.handler.handle_post(address, payload);
            }
            Message::Event {
                address, payload, ..
            } => {
                self.handler.handle_event(address, payload);
            }
            Message::Capabilities {
                address,
                capabilities,
                ..
            } => {
                self.handler.handle_capabilities(address, capabilities);
            }
            Message::Cancel { call_id, .. } => {
                self.server_calls.cancel(&call_id);
            }
            Message::Reply { id, payload, .. } => {
                self.client_requests
                    .dispatch_response(id, Response::Reply(payload));
            }
            Message::Error { id, error, .. } => {
                self.client_requests
                    .dispatch_response(id, Response::Error(error));
            }
            Message::Canceled { id, .. } => {
                self.client_requests
                    .dispatch_response(id, Response::Canceled);
            }
        }
    }
}
