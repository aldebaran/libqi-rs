mod router_handler;
mod server;

use self::router_handler::{PendingServiceMap, RouterHandler};
use crate::{
    messaging,
    object::{self, BoxObject, Object},
    service::{self, Info},
    service_directory::{self, ServiceDirectory},
    session::{
        self,
        authentication::{Authenticator, PermissiveAuthenticator},
    },
    value::{self, os::MachineId},
    Address, Error,
};
use futures::{stream, StreamExt, TryStreamExt};
use qi_value::KeyDynValueMap;
use router_handler::ArcRouterHandler;
use serde_with::serde_as;
use server::ServerSet;
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use tokio::{sync::Mutex, task};

pub struct Builder<Auth, Method, Body> {
    uid: Uid,
    authenticator: Auth,
    bind_addresses: Vec<Address>,
    pending_services: PendingServiceMap,
    method: Method,
    phantom_body: PhantomData<fn(Body) -> Body>,
}

impl Builder<PermissiveAuthenticator, (), value::BinaryFormattedValue> {
    pub fn new() -> Self {
        Builder::default()
    }
}

impl<Auth, Method, Body> Builder<Auth, Method, Body> {
    pub fn with_authenticator<NewAuth>(
        self,
        authenticator: NewAuth,
    ) -> Builder<NewAuth, Method, Body> {
        Builder {
            authenticator,
            uid: self.uid,
            bind_addresses: self.bind_addresses,
            pending_services: self.pending_services,
            method: self.method,
            phantom_body: PhantomData,
        }
    }

    pub fn with_body<NewBody>(self) -> Builder<Auth, Method, Body> {
        Builder {
            authenticator: self.authenticator,
            uid: self.uid,
            bind_addresses: self.bind_addresses,
            pending_services: self.pending_services,
            method: self.method,
            phantom_body: PhantomData,
        }
    }

    pub fn add_service<Name, Obj>(mut self, name: Name, object: Obj) -> Self
    where
        Name: std::string::ToString,
        Obj: Object + Send + Sync + 'static,
    {
        self.pending_services
            .add(name.to_string(), BoxObject::new(object));
        self
    }

    /// Binds the node to an address, accepting incoming connections on an
    /// endpoint at this address.
    pub fn bind(mut self, address: Address) -> Self {
        self.bind_addresses.push(address);
        self
    }

    /// Attaches the node to the space hosted at the given address.
    pub fn connect_to_space(
        self,
        address: Address,
        credentials: Option<KeyDynValueMap>,
    ) -> Builder<Auth, ConnectToSpace, Body> {
        Builder {
            authenticator: self.authenticator,
            uid: self.uid,
            bind_addresses: self.bind_addresses,
            pending_services: self.pending_services,
            method: ConnectToSpace {
                address,
                credentials,
            },
            phantom_body: PhantomData,
        }
    }

    /// Host a new space on this node.
    pub fn host_space<A>(self) -> Builder<Auth, HostSpace, Body> {
        Builder {
            authenticator: self.authenticator,
            uid: self.uid,
            bind_addresses: self.bind_addresses,
            pending_services: self.pending_services,
            method: HostSpace,
            phantom_body: PhantomData,
        }
    }
}

impl<Auth, Body> Builder<Auth, ConnectToSpace, Body>
where
    Auth: Authenticator + Send + Sync + Clone + 'static,
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    pub async fn start(self) -> Result<Node<service_directory::Client<Body>, Body>, Error> {
        let services = Arc::default();
        let handler = ArcRouterHandler::new(Arc::clone(&services));
        let server_set =
            ServerSet::new(handler.clone(), self.authenticator, self.bind_addresses).await?;
        let session_map = session::Map::new(handler);
        let session = session_map
            .get_or_create(
                service_directory::SERVICE_NAME,
                [self.method.address.into()],
                self.method.credentials.unwrap_or_default(),
            )
            .await?;
        let service_directory = service_directory::Client::new(session);
        let mut node = Node {
            uid: self.uid,
            services,
            session_map,
            service_directory,
            server_set,
        };
        node.init_services(self.pending_services).await?;
        Ok(node)
    }
}

impl<Auth, Method, Body> Builder<Auth, Method, Body> {}

impl<Auth, Method, Body> Default for Builder<Auth, Method, Body>
where
    Auth: Default,
    Method: Default,
{
    fn default() -> Self {
        Self {
            uid: Default::default(),
            authenticator: Default::default(),
            bind_addresses: Default::default(),
            pending_services: Default::default(),
            method: Default::default(),
            phantom_body: Default::default(),
        }
    }
}

impl<Auth, Method, Body> std::fmt::Debug for Builder<Auth, Method, Body>
where
    Auth: std::fmt::Debug,
    Method: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("uid", &self.uid)
            .field("authenticator", &self.authenticator)
            .field("bind_addresses", &self.bind_addresses)
            .field("pending_services", &self.pending_services)
            .field("method", &self.method)
            .finish()
    }
}

pub struct Node<SD, Body> {
    uid: Uid,
    services: Arc<Mutex<RouterHandler<Body>>>,
    session_map: session::Map<Body, ArcRouterHandler<Body>>,
    service_directory: SD,
    server_set: ServerSet,
}

impl<SD, Body> Node<SD, Body>
where
    SD: ServiceDirectory + Clone + Send + 'static,
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    async fn init_services(&mut self, pending_services: PendingServiceMap) -> Result<(), Error> {
        let mut server_endpoints_receiver = self.server_set.endpoints_receiver().clone();

        // Register each service to the directory, and mark them as ready.
        let server_endpoints =
            endpoints_to_client_targets(&server_endpoints_receiver.borrow_and_update());
        stream::iter(pending_services)
            .map(Ok)
            .try_for_each_concurrent(None, |(service_name, service_object)| {
                async {
                    let mut info = service::Info::process_local(
                        service_name.clone(),
                        service::UNSPECIFIED_ID,
                        server_endpoints.clone(),
                        self.uid.clone(),
                        service_object.uid(),
                    );
                    // Registering the service to the directory gets us a service ID, that we can use to
                    // update the local service info. With it, we can also index the service to the
                    // messaging handler so that it can start treating requests for that service.
                    // Consequently, we can notify the service directory of the readiness of the service.
                    let service_id = self.service_directory.register_service(&info).await?;
                    info.id = service_id;
                    self.services
                        .lock()
                        .await
                        .insert(service_name, info, service_object);
                    self.service_directory.service_ready(service_id).await?;
                    Ok::<_, Error>(())
                }
            })
            .await?;

        let services = Arc::clone(&self.services);
        let service_directory = self.service_directory.clone();
        // Update services info to the service directory whenever the server endpoints change.
        task::spawn(async move {
            while let Ok(()) = server_endpoints_receiver.changed().await {
                let server_endpoints =
                    endpoints_to_client_targets(&server_endpoints_receiver.borrow_and_update());
                for service_info in services.lock().await.info_mut() {
                    service_info.endpoints = server_endpoints.clone();
                    if let Err(_err) = service_directory.update_service_info(&*service_info).await {
                        // TODO: log the failure
                    }
                }
            }
        });
        Ok(())
    }
}

impl<SD, Body> Node<SD, Body>
where
    SD: ServiceDirectory,
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    pub async fn service(&self, name: &str) -> Result<impl Object + Clone, Error> {
        let service = self.service_directory.service(name).await?;
        let session = self
            .session_map
            .get_or_create(
                name,
                sort_service_endpoints(&service),
                // Connecting to service nodes of a space should not require credentials.
                Default::default(),
            )
            .await?;
        let object = object::Proxy::connect(
            service.id(),
            service::MAIN_OBJECT_ID,
            service.object_uid(),
            session,
        )
        .await?;
        Ok(object)
    }

    pub fn service_directory(&self) -> &SD {
        &self.service_directory
    }
}

impl<SD, Body> std::fmt::Debug for Node<SD, Body>
where
    SD: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttachedNode")
            .field("services", &self.services)
            .field("session_map", &self.session_map)
            .field("service_directory", &self.service_directory)
            .finish()
    }
}

fn sort_service_endpoints(service: &Info) -> Vec<session::Target> {
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

fn endpoints_to_client_targets(endpoints: &HashMap<Address, Vec<Address>>) -> Vec<session::Target> {
    let mut targets: Vec<_> = endpoints
        .values()
        .flatten()
        .copied()
        .map(session::Target::from)
        .collect();
    targets.sort();
    targets.dedup();
    targets
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    qi_macros::Valuable,
    serde_with::SerializeDisplay,
    serde_with::DeserializeFromStr,
)]
#[serde_as]
#[qi(value(crate = "crate::value", transparent))]
pub struct Uid(String);

impl Uid {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for Uid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Uid {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self::from_string(s.to_owned()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConnectToSpace {
    address: Address,
    credentials: Option<KeyDynValueMap>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostSpace;
