mod server;
mod services;
mod session_cache;

use std::sync::Arc;

pub use self::server::BindError;
use self::{
    server::Server,
    services::{PendingServices, RegisteredServices},
};
use crate::{
    object::{self, BoxObject, Object},
    os::MachineId,
    service::{self, Info},
    service_directory::ServiceDirectory,
    session, value, Error, Space,
};
use async_trait::async_trait;
use qi_messaging::Address;
use tower::Service;

#[async_trait]
pub trait Node {
    async fn add_service<S, O>(&mut self, name: S, object: O) -> Result<(), Error>
    where
        S: ToString,
        O: Object + Sync + Send + 'static;

    async fn remove_service(&mut self, name: &str) -> Result<BoxObject, Error>;
}

#[derive(Debug)]
pub struct DetachedNode {
    server: Server,
    services: PendingServices,
}

impl DetachedNode {
    fn new() -> Self {
        Self {
            server: server::Server::new(),
            services: PendingServices::default(),
        }
    }

    /// Bind the node to an address, accepting incoming connections on an
    /// endpoint at this address.
    pub fn listen(mut self, address: Address) -> Self {
        self.server.bind(address);
        self
    }

    /// Attaches the node to the space hosted at the given address.
    ///
    /// All services added to this node are registered on the space. Any
    /// connection or registration failure causes the attachment to stop.
    pub async fn attach_to_space(
        self,
        address: Address,
        authentication_parameters: session::authentication::Parameters,
    ) -> Result<AttachedNode, Error> {
        todo!()
        // let service = MessagingService::new(registered_services_receiver);
        // let session_registry = session::Registry::new(self.service.clone());
        // let sd_session = session_registry
        //     .get(
        //         service_directory::SERVICE_NAME,
        //         parameters.session_references,
        //         parameters.credentials,
        //     )
        //     .await?;
        // let service_directory = service_directory::Client::new(sd_session);
        // let endpoints = self
        //     .endpoints()
        //     .iter()
        //     .copied()
        //     .map(Into::into)
        //     .collect::<Vec<_>>();
        // for (name, object) in self.services {
        //     let info = Info::process_local(
        //         name.clone(),
        //         ServiceId::default(),
        //         endpoints.clone(),
        //         service_directory.id(),
        //         object.uid(),
        //     );
        //     let id = service_directory.register_service(&info).await?;
        //     self.registered_services.send_modify(move |services| {
        //         services.insert(
        //             id,
        //             (
        //                 name,
        //                 services::Objects::with_service_main_object(object.into()),
        //             ),
        //         );
        //     });
        //     service_directory.service_ready(id).await?;
        // }
        // Ok(Node {
        //     service: self.service,
        //     registered_services_sender: self.registered_services,
        //     session_registry,
        //     service_directory,
        // })
    }

    /// Host a new space on this node.
    fn host_space<A>(self, authenticator: A) -> AttachedNode {
        todo!()
    }
}

#[async_trait]
impl Node for DetachedNode {
    /// Add a named service to the node.
    async fn add_service<S, O>(&mut self, name: S, object: O) -> Result<(), Error>
    where
        S: ToString,
        O: Object + Sync + Send + 'static,
    {
        self.services.add(name.to_string(), object)
    }

    async fn remove_service(&mut self, name: &str) -> Result<BoxObject, Error> {}
}

pub struct AttachedNode {
    service_directory: Arc<dyn ServiceDirectory>,
    services: RegisteredServices,
    session_cache: SessionCache,
    server: Server,
}

impl std::fmt::Debug for AttachedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttachedNode")
            .field("services", &self.services)
            .field("server", &self.server)
            .finish()
    }
}

#[async_trait]
impl Space for AttachedNode {
    async fn service(&self, name: &str) -> Result<BoxObject, Error> {
        let service = self.service_directory.service(name).await?;
        let session = self
            .session_cache
            .get(
                name,
                sort_service_endpoints(&service),
                // Connecting to service nodes of a space should not require credentials.
                Default::default(),
            )
            .await?;
        let object = object::Client::connect(
            service.id(),
            service::MAIN_OBJECT_ID,
            service.object_uid(),
            session,
        );
        Ok(Box::new(object))
    }

    fn service_directory(&self) -> &dyn ServiceDirectory {
        self.service_directory.as_ref()
    }
}

fn sort_service_endpoints(service: &Info) -> Vec<session::Reference> {
    let service_is_local = service.machine_id() == MachineId::local();
    let mut endpoints = service.endpoints().to_vec();
    endpoints.sort_by_cached_key(|endpoint| {
        (
            endpoint.is_service_relative(),
            service_is_local && endpoint.is_machine_local(),
        )
    });
    endpoints
}
