use crate::{
    binary_value::BinaryValue,
    messaging::{
        message::{self, OnewayRequest},
        Address,
    },
    session::{authentication::Authenticator, Session},
    Error,
};
use futures::{stream, Sink, StreamExt};
use std::{collections::HashMap, future::Future};
use tokio::sync::watch;

#[derive(Debug)]
pub(super) struct Server {
    endpoints: watch::Receiver<HashMap<Address, Vec<Address>>>,
}

impl Server {
    pub(super) fn new<CallHandler, OnewaySink, Auth>(
        call_handler: CallHandler,
        oneway_sink: OnewaySink,
        authenticator: Auth,
        addresses: impl IntoIterator<Item = Address>,
    ) -> (Self, impl Future<Output = ()>)
    where
        CallHandler: tower::Service<(message::Address, BinaryValue), Response = BinaryValue, Error = Error>
            + Clone,
        OnewaySink: Sink<(message::Address, OnewayRequest<BinaryValue>), Error = Error> + Clone,
        Auth: Authenticator + Clone + Send + Sync + 'static,
    {
        let (endpoints_sender, endpoints_receiver) = watch::channel(Default::default());
        (
            Self {
                endpoints: endpoints_receiver,
            },
            stream::iter(addresses).for_each_concurrent(None, move |address| {
                let endpoints_sender = endpoints_sender.clone();
                let authenticator = authenticator.clone(); // TODO: Use ref instead of cloning ?
                let call_handler = call_handler.clone();
                let oneway_sink = oneway_sink.clone();
                async move {
                    if let Ok((server, endpoints)) =
                        Session::bind_server(address, authenticator, call_handler, oneway_sink)
                            .await
                    {
                        endpoints_sender.send_modify(|ep| {
                            ep.insert(address, endpoints);
                        });
                        server.await;
                        endpoints_sender.send_modify(|ep| {
                            ep.remove(&address);
                        });
                    }
                }
            }),
        )
    }

    pub(super) fn endpoints(&self) -> Vec<Address> {
        self.endpoints
            .borrow()
            .values()
            .flatten()
            .copied()
            .collect()
    }
}
