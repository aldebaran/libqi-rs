use crate::{
    object::{self, BoxObject, Object},
    os::MachineId,
    service::{self, Info},
    service_directory::{self, ServiceDirectory},
    session, space, Address, Error, Space,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::future::BoxFuture;
use qi_value::{ObjectId, ServiceId};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{watch, Mutex};

#[async_trait]
pub trait Node {
    /// Adds the given object as a named service on this node.
    ///
    /// If the node is attached to a space, the service is registered in that
    /// space. If the registration fails, returns an error result with the
    /// service and the error.
    async fn add_service<O>(&self, name: String, object: O) -> Result<(), AddServiceError<O>>
    where
        O: Object + Sync + Send + 'static;

    /// Removes the service with the given name from the node.
    ///
    /// Optionally returns the service boxed object if as service with this name
    /// exists.
    ///
    /// As soon as this function returns, the service is inaccessible from any
    /// nodes of the space this node is attached to.
    async fn remove_service(&self, name: &str) -> Option<BoxObject<'static>>;

    /// Binds the node to an address, accepting incoming connections on an
    /// endpoint at this address.
    async fn bind(&self, address: Address) -> Result<(), BindError>;

    fn endpoints(&self) -> Vec<Address>;
}

#[derive(Debug, thiserror::Error)]
#[error("error adding the service to this node")]
pub struct AddServiceError<T> {
    object: T,
    source: Error,
}

impl<T> AddServiceError<T> {
    pub fn into_object(self) -> T {
        self.object
    }
}

#[derive(Debug, thiserror::Error)]
#[error("error binding node to an address")]
pub enum BindError {}

/// Creates a new detached node.
pub fn create() -> DetachedNode {
    DetachedNode::new()
}

#[derive(Debug)]
pub struct DetachedNode {
    /// The service of the node that handles incoming messages from any
    /// sessions.
    service: Arc<MessagingService>,

    /// The list of services added to this node.
    services: Mutex<Services>,

    /// The list of registered services to communicate with the node messaging
    /// service.
    registered_services: watch::Sender<RegisteredServices>,
}

impl DetachedNode {
    fn new() -> Self {
        let (registered_services_sender, registered_services_receiver) =
            watch::channel(RegisteredServices::new());
        Self {
            service: Arc::new(MessagingService {
                services: registered_services_receiver,
            }),
            services: Mutex::default(),
            registered_services: registered_services_sender,
        }
    }

    /// Attaches to an existing space hosted by another node by connecting to it
    /// with the given parameters.
    ///
    /// All services added to this node are registered on the space. Any
    /// connection or registration failure causes the attachment to stop.
    pub async fn attach_space(self, parameters: space::Parameters) -> Result<AttachedNode, Error> {
        let session_registry = session::Registry::new(Arc::clone(&self.service));
        let sd_session = session_registry
            .get(
                service_directory::SERVICE_NAME,
                parameters.session_references,
                parameters.credentials,
            )
            .await?;
        let service_directory = service_directory::Client::new(sd_session);
        let endpoints = self
            .endpoints()
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        for (name, object) in self.services.into_inner().0 {
            let info = Info::process_local(
                name.clone(),
                ServiceId::default(),
                endpoints.clone(),
                service_directory.id(),
                object.uid(),
            );
            let id = service_directory.register_service(&info).await?;
            self.registered_services.send_modify(move |services| {
                services.insert(id, (name, Objects::with_service_main_object(object)));
            });
            service_directory.service_ready(id).await?;
        }
        Ok(AttachedNode {
            service: self.service,
            registered_services_sender: self.registered_services,
            session_registry,
            service_directory,
        })
    }

    /// Hosts a new space on this node, and consequently attaches to it.
    fn host_space(self) -> Result<HostNode, Error> {
        todo!()
    }
}

#[async_trait]
impl Node for DetachedNode {
    async fn add_service<O>(&self, name: String, object: O) -> Result<(), AddServiceError<O>>
    where
        O: Object + Sync + Send + 'static,
    {
        use std::collections::hash_map::Entry;
        match self.services.lock().await.0.entry(name.to_owned()) {
            Entry::Occupied(entry) => Err(AddServiceError {
                object,
                source: Error::ServiceExists(entry.key().clone()),
            }),
            Entry::Vacant(entry) => {
                entry.insert(Box::new(object));
                Ok(())
            }
        }
    }

    async fn remove_service(&self, name: &str) -> Option<BoxObject<'static>> {
        self.services.lock().await.0.remove(name)
    }

    async fn bind(&self, address: Address) -> Result<(), BindError> {
        todo!()
    }

    fn endpoints(&self) -> Vec<Address> {
        Vec::new()
    }
}

#[derive(Debug)]
pub struct AttachedNode {
    service: Arc<MessagingService>,
    registered_services_sender: watch::Sender<RegisteredServices>,
    session_registry: session::Registry<Arc<MessagingService>>,
    service_directory: service_directory::Client,
}

#[async_trait]
impl Space for AttachedNode {
    type ServiceDirectory = service_directory::Client;

    async fn service(&self, name: &str) -> Result<BoxObject, Error> {
        use crate::service_directory::ServiceDirectory;
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

#[derive(Debug)]
pub struct HostNode;

#[derive(Default)]
struct Services(HashMap<String, BoxObject<'static>>);

impl std::fmt::Debug for Services {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.0.keys().map(|name| (name, "Object")))
            .finish()
    }
}

type RegisteredServices = HashMap<ServiceId, (String, Objects)>;

#[derive(Default)]
struct Objects(HashMap<ObjectId, BoxObject<'static>>);

impl Objects {
    fn with_service_main_object(object: BoxObject<'static>) -> Self {
        Self(HashMap::from_iter([(service::MAIN_OBJECT_ID, object)]))
    }
}

impl std::fmt::Debug for Objects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.0.keys().map(|id| (id, "Object")))
            .finish()
    }
}

#[derive(Debug)]
struct MessagingService {
    services: watch::Receiver<RegisteredServices>,
}

impl qi_messaging::Service for MessagingService {
    fn call(
        &self,
        call: qi_messaging::Call,
    ) -> BoxFuture<'static, Result<Bytes, qi_messaging::Error>> {
        todo!()
    }

    fn post(
        &self,
        post: qi_messaging::Post,
    ) -> BoxFuture<'static, Result<(), qi_messaging::Error>> {
        todo!()
    }

    fn event(
        &self,
        event: qi_messaging::Event,
    ) -> BoxFuture<'static, Result<(), qi_messaging::Error>> {
        todo!()
    }
}
