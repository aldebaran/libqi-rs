mod messaging_service;
mod server;
mod services;

use self::messaging_service::MessagingService;
pub use self::server::BindError;
use crate::{
    authentication,
    object::{self, BoxObject, Object},
    os::MachineId,
    service::{self, Info},
    service_directory::{self, ServiceDirectory},
    session, value, Address, Error, PermissiveAuthenticator, Space,
};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Builder {
    server: server::Builder<PermissiveAuthenticator>,
    services: Services,
}

impl Builder {
    fn new() -> Self {
        Self {
            server: server::Builder::new(),
            services: Services::default(),
        }
    }

    /// Add a named service to the node.
    pub fn add_service<O>(mut self, name: String, object: O) -> Self
    where
        O: Object + Sync + Send + 'static,
    {
        self.services.0.insert(name, Box::new(object));
        self
    }

    /// Bind the node to an address, accepting incoming connections on an
    /// endpoint at this address.
    pub fn listen(mut self, address: Address) -> Self {
        self.server.bind(address);
        self
    }

    /// Attach the node to an existing space.
    pub fn attach_to_space(self, address: Address) -> AttachBuilder {
        AttachBuilder {
            server: self.server,
            services: self.services,
            address,
            authentication_parameters: authentication::Parameters::new(),
        }
    }

    /// Host a new space on this node.
    fn host_space(self) -> HostBuilder<PermissiveAuthenticator> {
        HostBuilder {
            server: self.server,
            services: self.services,
        }
    }
}

#[derive(Debug)]
pub struct AttachBuilder {
    services: Services,
    server: server::Builder<PermissiveAuthenticator>,
    address: Address,
    authentication_parameters: authentication::Parameters,
}

impl AttachBuilder {
    pub fn set_authentication_parameter<V>(mut self, name: String, value: V) -> Self
    where
        V: value::IntoValue<'static>,
    {
        self.authentication_parameters
            .insert(name, value.into_value());
        self
    }

    /// Builds the node, attaching it to the space hosted by another node with
    /// the configured parameters.
    ///
    /// All services added to this node are registered on the space. Any
    /// connection or registration failure causes the attachment to stop.
    pub async fn build(self) -> Result<Node<service_directory::Client>, Error> {
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
        todo!()
    }
}

#[derive(Debug)]
pub struct HostBuilder<A> {
    server: server::Builder<A>,
    services: Services,
}

impl<A> HostBuilder<A> {
    pub fn set_authenticator<A2>(mut self, authenticator: A2) -> HostBuilder<A2> {
        HostBuilder {
            server: self.server.set_authenticator(authenticator),
            services: self.services,
        }
    }

    pub async fn build(self) -> Result<Node<service_directory::Client>, Error> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Node<SD> {
    session_registry: session::Cache<MessagingService>,
    service_directory: SD,
}

impl<SD> Node<SD> {
    fn endpoints(&self) -> &[Address] {
        todo!()
    }
}

#[async_trait]
impl<SD> Space for Node<SD>
where
    SD: ServiceDirectory + Send + Sync,
{
    type ServiceDirectory = SD;

    async fn service(&self, name: &str) -> Result<BoxObject, Error> {
        let service = self.service_directory.service(name).await?;
        let session = self
            .session_registry
            .get(
                name,
                sort_service_endpoints(&service),
                // Connecting to service nodes of a space should not require credentials.
                Default::default(),
            )
            .await?;
        let meta_object =
            object::fetch_meta(&session, service.id(), service::MAIN_OBJECT_ID).await?;
        let object = object::Client::new(
            service.id(),
            service::MAIN_OBJECT_ID,
            service.object_uid(),
            meta_object,
            session,
        );
        Ok(Box::new(object))
    }

    fn service_directory(&self) -> &Self::ServiceDirectory {
        &self.service_directory
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

#[derive(Default)]
pub(super) struct Services(HashMap<String, BoxObject<'static>>);

impl std::fmt::Debug for Services {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.keys()).finish()
    }
}
