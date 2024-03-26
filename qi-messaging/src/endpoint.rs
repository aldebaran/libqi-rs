pub use self::outgoing::OutgoingMessages;
use crate::{
    client,
    id_factory::SharedIdFactory,
    message::{Address, OnewayRequest},
    server, Client, Message,
};
use futures::{Sink, TryStream};

#[allow(clippy::type_complexity)]
pub fn endpoint<Msgs, Svc, Snk, InBody, OutBody>(
    messages: Msgs,
    service: Svc,
    oneway_requests_sink: Snk,
) -> (
    Client<OutBody, InBody>,
    OutgoingMessages<Msgs, Svc, Snk, InBody, OutBody>,
)
where
    Msgs: TryStream<Ok = Message<InBody>>,
    Msgs::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    Svc: tower_service::Service<(Address, InBody), Response = OutBody>,
    Svc::Error: std::string::ToString + Into<Box<dyn std::error::Error + Send + Sync>>,
    Snk: Sink<(Address, OnewayRequest<InBody>)>,
    Snk::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    InBody: Send,
    OutBody: Send,
{
    let id_factory = SharedIdFactory::new();
    let (client, client_requests) = client::pair(id_factory, 1);
    let server_responses = server::Responses::new(service);
    let outgoing = OutgoingMessages::new(
        messages,
        server_responses,
        oneway_requests_sink,
        client_requests,
    );
    (client, outgoing)
}

mod outgoing;
