pub(crate) use self::outgoing::OutgoingMessages;
use crate::{
    client,
    id_factory::SharedIdFactory,
    message::{Address, OnewayRequest},
    server, Client, Error, Message,
};
use futures::{stream::FusedStream, Sink, TryStream};

#[allow(clippy::type_complexity)]
pub fn endpoint<Msgs, Handler, Snk, InBody, OutBody>(
    messages: Msgs,
    handker: Handler,
    oneway_requests_sink: Snk,
) -> (
    Client<OutBody, InBody>,
    impl FusedStream<Item = Result<Message<OutBody>, Error>>,
)
where
    Msgs: TryStream<Ok = Message<InBody>>,
    Msgs::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    Handler: tower_service::Service<(Address, InBody), Response = OutBody>,
    Handler::Error: std::string::ToString + Into<Box<dyn std::error::Error + Send + Sync>>,
    Snk: Sink<(Address, OnewayRequest<InBody>)>,
    Snk::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    InBody: Send,
    OutBody: Send,
{
    let id_factory = SharedIdFactory::new();
    let (client, client_requests) = client::pair(id_factory, 1);
    let server_responses = server::Responses::new(handker);
    // TODO: Use stream! instead of defining the stream ourselves
    let outgoing = OutgoingMessages::new(
        messages,
        server_responses,
        oneway_requests_sink,
        client_requests,
    );
    (client, outgoing)
}

mod outgoing;
