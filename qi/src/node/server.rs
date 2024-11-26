use crate::{
    error::HandlerError,
    messaging::{self, Address},
    session::{authentication::Authenticator, Session},
};
use std::collections::HashMap;
use tokio::{sync::watch, task};

/// A set of servers with their aggregated endpoints.
///
/// Drops all server tasks when the set is dropped.
#[derive(Debug)]
pub(super) struct ServerSet {
    endpoints: watch::Receiver<HashMap<Address, Vec<Address>>>,
    #[allow(dead_code)]
    server_tasks: task::JoinSet<()>,
}

impl ServerSet {
    /// Instantiates a set of servers for a list of addresses and aggregates their endpoints.
    ///
    /// For each server created, a task is spawned that will track changes to its endpoints.
    ///
    /// If any server fails to bind to its address, then the future terminates with an error and all
    /// created servers are stopped.
    pub(super) async fn new<Handler, Auth, Body>(
        handler: Handler,
        authenticator: Auth,
        addresses: impl IntoIterator<Item = Address>,
    ) -> Result<Self, std::io::Error>
    where
        Handler: messaging::Handler<Body, Error = HandlerError> + Send + Sync + Clone + 'static,
        Auth: Authenticator + Clone + Send + Sync + 'static,
        Body: messaging::Body + Send + 'static,
        Body::Error: Send + Sync + 'static,
    {
        let (set_endpoints_sender, set_endpoints_receiver) = watch::channel(Default::default());
        let mut server_tasks = task::JoinSet::new();
        for address in addresses {
            let mut server =
                Session::server(address, authenticator.clone(), handler.clone()).await?;
            let set_endpoints_sender = set_endpoints_sender.clone();
            server_tasks.spawn(async move {
                let server_endpoints = server.endpoints_receiver();
                while let Ok(()) = server_endpoints.changed().await {
                    set_endpoints_sender.send_modify(|endpoints: &mut HashMap<_, _>| {
                        let server_endpoints = server_endpoints.borrow_and_update();
                        endpoints.insert(server_endpoints.0, server_endpoints.1.clone());
                    });
                }
            });
        }
        Ok(ServerSet {
            endpoints: set_endpoints_receiver,
            server_tasks,
        })
    }

    pub(super) fn endpoints_receiver(
        &mut self,
    ) -> &mut watch::Receiver<HashMap<Address, Vec<Address>>> {
        &mut self.endpoints
    }
}
