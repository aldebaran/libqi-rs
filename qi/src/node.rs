mod address;

pub use self::address::Address;
use crate::{
    machine_id::MachineId,
    object::{self, BoxObject, Object},
    sd,
    service::{self, ServiceInfo},
    session, Error,
};
use bytes::Bytes;
use futures::future::BoxFuture;
use qi_messaging as messaging;
use qi_value::{ObjectId, ServiceId, Value};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::watch;

pub trait Node {
    fn add_service<O>(name: String, object: O)
    where
        O: Object;

    fn remove_service(name: &str);
}

/// Creates a new isolated node.
pub fn create() -> IsolatedNode {
    let (services_sender, services_receiver) = watch::channel(HashMap::new());
    let server = Arc::new(Server {
        services: services_receiver,
    });
    let node = IsolatedNode {
        server,
        services: services_sender,
    };
    node
}

pub struct IsolatedNode {
    server: Arc<Server>,
    services: watch::Sender<ServicesMap>,
}

impl IsolatedNode {
    /// Attaches to an existing space hosted by a node with the given configuration.
    pub async fn attach_space(self, config: Config) -> Result<AttachedNode, Error> {
        let sessions = session::Cache::new();
        let session = sessions
            .get(config, sd::SERVICE_NAME, Arc::clone(&self.server))
            .await?;
        let service_directory = sd::Client::new(session);
        Ok(AttachedNode {
            service: self.server,
            sessions,
            service_directory,
        })
    }

    /// Hosts a new space on this node.
    fn host_space(self) -> Result<HostNode, Error> {
        todo!()
    }
}

impl std::fmt::Debug for IsolatedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct AttachedNode {
    service: Arc<Server>,
    sessions: session::Cache,
    service_directory: sd::Client,
}

impl AttachedNode {
    pub fn service_directory(&self) -> &sd::Client {
        &self.service_directory
    }

    pub async fn service(&self, name: &str) -> Result<Box<dyn Object + Send + Sync>, Error> {
        use crate::sd::ServiceDirectory;
        let service = self.service_directory.service_info(name).await?;
        let session = self
            .sessions
            .get(
                Config {
                    addresses: sort_endpoints(&service),
                    // Connecting to service nodes of a space should not require credentials.
                    credentials: Default::default(),
                },
                name,
                Arc::clone(&self.service),
            )
            .await?;
        let object = object::Client::new(service.service_id, service::MAIN_OBJECT_ID, session);
        Ok(Box::new(object))
    }
}

#[derive(Debug)]
pub struct HostNode;

#[derive(Default, Clone, Debug)]
pub struct Config {
    /// Session addresses that may be used to connect to the node.
    pub(crate) addresses: Vec<session::Address>,
    /// Credentials required to authenticate to the node control server.
    pub(crate) credentials: session::authentication::Parameters,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_address<A>(mut self, address: A) -> Self
    where
        A: Into<Address>,
    {
        self.addresses.push(session::Address::Node(address.into()));
        self
    }

    pub fn add_credentials_parameter(mut self, key: String, value: Value<'static>) -> Self {
        self.credentials.insert(key, value);
        self
    }
}

type ServicesMap = HashMap<ServiceId, HashMap<ObjectId, BoxObject<'static>>>;

pub(crate) struct Server {
    services: watch::Receiver<ServicesMap>,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server").finish()
    }
}

impl messaging::Service for Server {
    fn call(&self, call: messaging::Call) -> BoxFuture<'static, Result<Bytes, messaging::Error>> {
        todo!()
    }

    fn post(&self, post: messaging::Post) -> BoxFuture<'static, Result<(), messaging::Error>> {
        todo!()
    }

    fn event(&self, event: messaging::Event) -> BoxFuture<'static, Result<(), messaging::Error>> {
        todo!()
    }
}

fn sort_endpoints(service: &ServiceInfo) -> Vec<session::Address> {
    let service_is_local = &service.machine_id == MachineId::local();
    let mut endpoints = service.endpoints.clone();
    endpoints.sort_by_cached_key(|endpoint| {
        (
            endpoint.is_relative(),
            service_is_local && endpoint.is_machine_local(),
        )
    });
    endpoints
}
