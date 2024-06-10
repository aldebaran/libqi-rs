pub(crate) use self::outgoing::OutgoingMessages;
use crate::{
    client,
    id_factory::SharedIdFactory,
    message::{Address, OnewayRequest},
    server, Client, Error, Message,
};
use futures::{stream::FusedStream, Sink, SinkExt, Stream, StreamExt, TryStream};
use std::future::Future;

pub fn dispatch<MsgStream, CallHandler, OnewaySink, InBody, OutBody>(
    messages: MsgStream,
    call_handler: CallHandler,
    oneway_sink: OnewaySink,
) -> (
    Client<OutBody, InBody>,
    impl FusedStream<Item = Result<Message<OutBody>, Error>>,
)
where
    MsgStream: TryStream<Ok = Message<InBody>>,
    MsgStream::Error: std::error::Error + Sync + Send + 'static,
    CallHandler: tower_service::Service<(Address, InBody), Response = OutBody>,
    CallHandler::Error: std::error::Error + Sync + Send + 'static,
    OnewaySink: Sink<(Address, OnewayRequest<InBody>)>,
    OnewaySink::Error: std::error::Error + Sync + Send + 'static,
    InBody: Send,
    OutBody: Send,
{
    let id_factory = SharedIdFactory::new();
    let (client, client_requests) = client::pair(id_factory, 1);
    let server_responses = server::Responses::new(call_handler);
    // TODO: Use stream! instead of defining the stream ourselves
    let outgoing = OutgoingMessages::new(messages, server_responses, oneway_sink, client_requests);
    (client, outgoing)
}

pub fn start<MsgStream, MsgSink, CallHandler, OnewaySink, ReadBody, WriteBody>(
    messages_stream: MsgStream,
    messages_sink: MsgSink,
    call_handler: CallHandler,
    oneway_sink: OnewaySink,
) -> (
    Client<WriteBody, ReadBody>,
    impl Future<Output = Result<(), Error>>,
)
where
    MsgStream: Stream<Item = Result<Message<ReadBody>, Error>>,
    MsgSink: Sink<Message<WriteBody>, Error = Error>,
    CallHandler: tower_service::Service<(Address, ReadBody), Response = WriteBody>,
    CallHandler::Error: std::error::Error + Sync + Send + 'static,
    OnewaySink: Sink<(Address, OnewayRequest<ReadBody>)>,
    OnewaySink::Error: std::error::Error + Sync + Send + 'static,
    ReadBody: Send,
    WriteBody: Send,
{
    let (client, outgoing_messages) = dispatch(messages_stream, call_handler, oneway_sink);
    let connection = outgoing_messages.forward(messages_sink.sink_err_into());
    (client, connection)
}

mod outgoing;
