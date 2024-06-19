use crate::{
    messaging::{self, Address},
    session::{authentication::Authenticator, Session},
    value::BinaryValue,
    Error,
};
use futures::{stream, StreamExt};
use std::{collections::HashMap, future::Future};
use tokio::sync::watch;

pub(super) fn create<Handler, Auth>(
    handler: Handler,
    authenticator: Auth,
    addresses: impl IntoIterator<Item = Address>,
) -> (EndpointsRx, impl Future<Output = ()>)
where
    Handler: messaging::Handler<BinaryValue, Reply = BinaryValue, Error = Error> + Sync + Clone,
    Auth: Authenticator + Clone + Send + Sync + 'static,
{
    let (endpoints_sender, endpoints_receiver) = watch::channel(Default::default());
    (
        endpoints_receiver,
        stream::iter(addresses).for_each_concurrent(None, move |address| {
            let endpoints_sender = endpoints_sender.clone();
            let authenticator = authenticator.clone(); // TODO: Use ref instead of cloning ?
            let handler = handler.clone();
            async move {
                if let Ok((server, endpoints)) =
                    Session::bind_server(address, authenticator, handler).await
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

pub(super) type EndpointsRx = watch::Receiver<HashMap<Address, Vec<Address>>>;
