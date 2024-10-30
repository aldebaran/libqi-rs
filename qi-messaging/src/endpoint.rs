use crate::{
    client,
    id_factory::SharedIdFactory,
    message::{Oneway, Response},
    server, Client, Error, Handler, Message,
};
use async_stream::stream;
use futures::{stream::FusedStream, Sink, SinkExt, Stream, StreamExt, TryStream, TryStreamExt};
use qi_value::Dynamic;
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
pub fn dispatch<MsgStream, H, Body>(
    messages: MsgStream,
    handler: H,
) -> (
    Client<Body>,
    impl Stream<Item = Result<Message<Body>, Error>>,
)
where
    MsgStream: TryStream<Ok = Message<Body>>,
    MsgStream::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
    H: Handler<Body>,
    H::Error: std::string::ToString,
{
    let messages = messages
        .into_stream()
        .map_err(|err| Error::LinkLost(err.into()))
        .fuse();
    let id_factory = SharedIdFactory::new();
    let (client, mut client_requests) = client::pair(id_factory, 1);
    let outgoing = stream! {
        let mut server_calls = server::CallFutures::default();
        let mut messages = pin!(messages);
        loop {
            select! {
                // Process and dispatch incoming messages.
                Some(res_message) = messages.next(), if !messages.is_terminated() => {
                    match res_message {
                        Ok(message) => match message {
                            Message::Call { id, address, value } => {
                                let call_future = handler.call(address, value);
                                server_calls.push(id, address, call_future);
                            }
                            Message::Post { address, value, .. } => {
                                handler.oneway(address, Oneway::Post(value)).await;
                            }
                            Message::Event { address, value, .. } => {
                                handler.oneway(address, Oneway::Event(value)).await;
                            },
                            Message::Capabilities { address, capabilities, .. } => {
                                handler.oneway(address, Oneway::Capabilities(capabilities)).await;
                            },
                            Message::Cancel { call_id, .. } => {
                                server_calls.cancel(&call_id);
                            },
                            Message::Reply { id, value, .. } => {
                                client_requests.dispatch_response(id, Response::Reply(value));
                            },
                            Message::Error { id, error: Dynamic(error), .. } => {
                                client_requests.dispatch_response(id, Response::Error(error));
                            },
                            Message::Canceled { id, .. } => {
                                client_requests.dispatch_response(id, Response::Canceled);
                            },
                        },
                        Err(err) => yield Err(err),
                    }
                }
                Some(message) = server_calls.next(), if !server_calls.is_terminated() => {
                    yield Ok(message);
                }
                Some(message) = client_requests.next(), if !client_requests.is_terminated() => {
                    yield Ok(message);
                }
                else => {
                    break;
                }
            }
        }
    };
    (client, outgoing)
}

pub fn start<MsgStream, MsgSink, H, Body>(
    messages_stream: MsgStream,
    messages_sink: MsgSink,
    handler: H,
) -> (Client<Body>, impl Future<Output = Result<(), Error>>)
where
    MsgStream: TryStream<Ok = Message<Body>>,
    MsgStream::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
    MsgSink: Sink<Message<Body>, Error = Error>,
    H: Handler<Body>,
    H::Error: std::string::ToString,
{
    let (client, outgoing_messages) = dispatch(messages_stream, handler);
    let connection = outgoing_messages.forward(messages_sink.sink_err_into());
    (client, connection)
}
