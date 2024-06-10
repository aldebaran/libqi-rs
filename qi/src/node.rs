mod messaging_handler;
mod server;
mod service_map;
mod session_factory;

use self::{
    server::Server,
    service_map::{PendingServiceMap, ServiceMap},
};
use crate::{
    object::{self, BoxObject, Object},
    os::MachineId,
    service::{self, Info},
    service_directory::ServiceDirectory,
    session::{
        self,
        authentication::{self, Authenticator, PermissiveAuthenticator},
    },
    Error, Result, Space,
};
use async_trait::async_trait;
use messaging_handler::MessagingHandler;
use qi_messaging::Address;
use session_factory::SessionFactory;
use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;

#[async_trait]
pub trait Node {
    async fn add_service<S, O>(&mut self, name: String, object: O) -> Result<()>
    where
        O: Object + Sync + Send + 'static;

    async fn remove_service(&mut self, name: &str) -> Result<BoxObject>;
}

#[derive(Default, Debug)]
pub struct Builder<Auth> {
    authenticator: Auth,
    bind_addresses: Vec<Address>,
    services: PendingServiceMap,
}

impl<Auth> Builder<Auth>
where
    Auth: Authenticator + Send + Sync + Clone + 'static,
{
    pub fn new() -> Builder<PermissiveAuthenticator> {
        Builder::default()
    }

    pub fn with_authenticator<A>(self, authenticator: A) -> Builder<A> {
        Builder {
            authenticator,
            bind_addresses: self.bind_addresses,
            services: self.services,
        }
    }

    /// Bind the node to an address, accepting incoming connections on an
    /// endpoint at this address.
    pub fn bind(mut self, address: Address) -> Self {
        self.bind_addresses.push(address);
        self
    }

    /// Attaches the node to the space hosted at the given address.
    ///
    /// All services added to this node are registered on the space. Any
    /// connection or registration failure causes the attachment to stop.
    pub async fn attach_to_space(
        self,
        address: Address,
        authentication_parameters: authentication::Parameters<'_>,
    ) -> Result<AttachedNode> {
        todo!()
        // let services = Arc::default();
        // let handler = MessagingHandler::new(Arc::clone(&services));
        // let (server, server_connection) =
        //     server::Server::new(handler.clone(), handler, authenticator);
        // (
        //     Self {
        //         server,
        //         pending_services: Default::default(),
        //         services,
        //     },
        //     server_connection,
        // )

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
impl<A> Node for Builder<A>
where
    A: Send,
{
    /// Add a named service to the node.
    async fn add_service<S, O>(&mut self, name: String, object: O) -> Result<()>
    where
        O: Object + Sync + Send + 'static,
    {
        self.services.add(name, Box::new(object))
    }

    async fn remove_service(&mut self, name: &str) -> Result<BoxObject> {
        self.services
            .remove(name)
            .ok_or_else(|| Error::ServiceNotFound(name.to_string()))
    }
}

pub struct AttachedNode {
    service_directory: Arc<dyn ServiceDirectory + Send + Sync>,
    services: ServiceMap,
    session_factory: SessionFactory<MessagingHandler, MessagingHandler>,
    server: Server,
}

impl std::fmt::Debug for AttachedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttachedNode")
            .field("services", &self.services)
            .field("session_factory", &self.session_factory)
            .field("server", &self.server)
            .finish()
    }
}

#[async_trait]
impl Space for AttachedNode {
    async fn service(&self, name: &str) -> Result<BoxObject> {
        let service = self.service_directory.service(name).await?;
        let session = self
            .session_factory
            .establish(
                name,
                sort_service_endpoints(&service).iter(),
                // Connecting to service nodes of a space should not require credentials.
                Default::default(),
            )
            .await?;
        let object = object::Client::connect(
            service.id(),
            service::MAIN_OBJECT_ID,
            service.object_uid(),
            session,
        )
        .await?;
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
