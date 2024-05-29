use crate::{Authenticator, PermissiveAuthenticator};
use qi_messaging::Address;

pub(super) struct Server {
    endpoints: Vec<Address>,
    authenticator: Box<dyn Authenticator>,
}

impl Server {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn endpoints(&self) -> &[Address] {
        &self.endpoints
    }
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("endpoints", &self.endpoints)
            .finish()
    }
}

impl Default for Server {
    fn default() -> Self {
        Self {
            endpoints: Default::default(),
            authenticator: Box::new(PermissiveAuthenticator),
        }
    }
}

// impl<Svc, A> Server<Svc, A>
// where
//     Svc: qi_messaging::Service + Send + Clone + 'static,
//     A: session::Authenticator + Clone,
// {
//     pub(super) async fn bind(&mut self, address: Address) -> Result<(), BindError> {
//         let (server, endpoints) =
//             session::serve(address, self.service.clone(), self.authenticator.clone()).await?;
//         self.endpoints.extend(endpoints);
//         // Spawn a task that will poll for new sessions and move each of them into a new task
//         // that drives the connection to completion while keeping the client side alive.
//         self.server_tasks.spawn(server);
//         Ok(())
//     }
// }

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BindError(#[from] std::io::Error);
