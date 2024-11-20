use crate::{
    client,
    id_factory::SharedIdFactory,
    message::{FireAndForget, Response},
    server, Client, Message,
};
use async_stream::stream;
use either::Either;
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Sink, SinkExt, Stream, StreamExt, TryStream, TryStreamExt,
};
use std::{future::Future, pin::pin};
use tokio::select;

// Returns A stream of outgoing messages of an endpoint.
//
// It selects between two stream sides:
//   - incoming messages,
//   - client requests,
//
// Client requests are the source of outgoing messages of types Call, Post, Event, Capabilities and
// Cancel. They originate only from clients objects request channel but never from the incoming
// messages.
//
// Server responses are the source of outgoing messages of types Reply, Error and
// Canceled. They originate from a sequencing between incoming messages and handler calls.
//
// Incoming messages also have side-effects on the results sent to clients.
//
//                ┌───────────────────────┐
//                │                       │
//                │   Incoming Messages   │
//                │                       │
//                └───────────┬───────────┘
//                            │
//    ┌─────┬────┬─────┬──────┴──┬────────┬──────┬───────┐
//    │     │    │     │         │        │      │       │
//  Call Cancel Post Event Capabilities Reply Canceled Error
//    │     │    │     │         │        │      │       │              ┌─────────┐
// ┌──▼─────▼────▼─────▼─────────▼─────┬──▼──────▼───────▼──┐           │         ├┐
// │                                   │                    │           │ Clients ││
// │              Request              │      Response      │           │         ││
// │                                   │                    │           └┬────────┘│
// └──┬─────┬────┬─────┬─────────┬─────┴──┬──────┬───────┬──┘            └─────────┘
//    │     │    │     │         │        │      │       │     Call Cancel Post Event Capababilities
//  ┌─▼─────▼─┬──▼─────▼─────────▼────┐ ┌─▼──────▼───────▼─┐     │    │     │     │         │
//  │         │                       │ │                  │     │    │     │     │         │
//  │ Handler │        Oneway         │ │      Client      ◄─────┴────┴─────┴─────┴─────────┘
//  │  Calls  │         Sink          │ │     Requests     │
//  │         │                       │ │                  │
//  └────┬────┴───────────────────────┘ └────────┬─────────┘
//    Server                                     │
//   Responses                                   │
//       └───────────────┐      ┌────────────────┘
//                       │      │
//               ┌───────▼──────▼────────┐
//               │                       │
//               │   Outgoing Messages   │
//               │                       │
//               └───────────────────────┘
pub fn dispatch<MsgStream, Handler, Body>(
    messages: MsgStream,
    handler: Handler,
) -> (
    Client<Body>,
    impl Stream<Item = Result<Message<Body>, MsgStream::Error>>,
)
where
    MsgStream: TryStream<Ok = Message<Body>>,
    Handler: crate::Handler<Body>,
{
    let messages = messages.into_stream().fuse();
    let id_factory = SharedIdFactory::new();
    let (client, mut client_requests) = client::pair(id_factory, 1);
    let outgoing = stream! {
        let mut server_calls = server::CallFutures::default();
        let mut server_faf = FuturesUnordered::new();
        let mut messages = pin!(messages);
        loop {
            select! {
                // Process and dispatch incoming messages.
                Some(message) = messages.next(), if !messages.is_terminated() => {
                    match message {
                        Ok(message) => match message {
                            Message::Call { id, address, value } => {
                                let call_future = handler.call(address, value);
                                server_calls.push(id, address, call_future);
                            }
                            Message::Post { address, value, .. } => {
                                server_faf.push(handler.fire_and_forget(address, FireAndForget::Post(value)));
                            }
                            Message::Event { address, value, .. } => {
                                server_faf.push(handler.fire_and_forget(address, FireAndForget::Event(value)));
                            },
                            Message::Capabilities { address, capabilities, .. } => {
                                server_faf.push(handler.fire_and_forget(address, FireAndForget::Capabilities(capabilities)));
                            },
                            Message::Cancel { call_id, .. } => {
                                server_calls.cancel(&call_id);
                            },
                            Message::Reply { id, value, .. } => {
                                client_requests.dispatch_response(id, Response::Reply(value));
                            },
                            Message::Error { id, error, .. } => {
                                client_requests.dispatch_response(id, Response::Error(error));
                            },
                            Message::Canceled { id, .. } => {
                                client_requests.dispatch_response(id, Response::Canceled);
                            }
                        },
                        Err(err) => yield Err(err),
                    }
                }
                Some((message, stop)) = server_calls.next(), if !server_calls.is_terminated() => {
                    yield Ok(message);
                    if stop {
                        break;
                    }
                }
                Some(()) = server_faf.next(), if !server_faf.is_terminated() => {
                    // Nothing
                }
                message = client_requests.next() => {
                    match message {
                        Some(message) => yield Ok(message),
                        None => break,
                    }
                }
                else => {
                    break;
                }
            }
        }
    };
    (client, outgoing)
}

pub fn start<MsgStream, MsgSink, Handler, Body>(
    messages_stream: MsgStream,
    messages_sink: MsgSink,
    handler: Handler,
) -> (
    Client<Body>,
    impl Future<Output = Result<(), Either<MsgStream::Error, MsgSink::Error>>>,
)
where
    MsgStream: TryStream<Ok = Message<Body>>,
    MsgSink: Sink<Message<Body>>,
    Handler: crate::Handler<Body>,
{
    let (client, outgoing_messages) = dispatch(messages_stream, handler);
    let connection = outgoing_messages
        .map_err(Either::Left)
        .forward(messages_sink.sink_map_err(Either::Right));
    (client, connection)
}
