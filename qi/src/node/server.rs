use crate::{Address, PermissiveAuthenticator};

#[derive(Debug)]
pub(super) struct Builder<A> {
    authenticator: A,
    addresses: Vec<Address>,
}

impl Builder<PermissiveAuthenticator> {
    pub(super) fn new() -> Self {
        Self {
            authenticator: PermissiveAuthenticator,
            addresses: Vec::new(),
        }
    }
}

impl<A> Builder<A> {
    pub(super) fn set_authenticator<A2>(mut self, authenticator: A2) -> Builder<A2> {
        Builder {
            authenticator,
            addresses: self.addresses,
        }
    }

    pub(super) fn bind(&mut self, address: Address) {
        self.addresses.push(address)
    }
}

#[derive(Default, Debug)]
pub(super) struct Server {
    endpoints: Vec<Address>,
}

impl Server {
    // pub(super) fn new(service: Svc, authenticator: A) -> Self {
    //     Self {
    //         service,
    //         authenticator,
    //         endpoints: Vec::new(),
    //         server_tasks: JoinSet::new(),
    //     }
    // }

    pub(super) fn endpoints(&self) -> &[Address] {
        &self.endpoints
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
