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
use futures::{future::FusedFuture, FutureExt};
use qi_value::KeyDynValueMap;
use router_handler::ArcRouterHandler;
use serde_with::serde_as;
use server::EndpointsRx;
use std::{future::Future, marker::PhantomData, pin::pin, sync::Arc};
use tokio::{select, sync::Mutex};

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
    pub async fn start(
        self,
    ) -> Result<
        (
            Node<service_directory::Client<Body>, Body>,
            impl Future<Output = ()>,
        ),
        Error,
    > {
        let services = Arc::default();
        let handler = router_handler::ArcRouterHandler::new(Arc::clone(&services));
        let (server_endpoints, server_task) =
            server::create(handler.clone(), self.authenticator, self.bind_addresses);
        let (session_map, session_connections) = session::Map::new(handler);
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
        };
        let task = node
            .init(
                self.pending_services,
                server_task,
                session_connections,
                server_endpoints,
            )
            .await?;
        Ok((node, task))
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
}

impl<SD, Body> Node<SD, Body>
where
    SD: ServiceDirectory + Clone,
    Body: messaging::Body + Send,
    Body::Error: Send + Sync + 'static,
{
    async fn init(
        &mut self,
        pending_services: PendingServiceMap,
        server_task: impl Future<Output = ()>,
        session_connections: impl Future<Output = ()>,
        mut server_endpoints: EndpointsRx,
    ) -> Result<impl Future<Output = ()>, Error> {
        // Register each service to the directory, and mark them as ready.
        for (service_name, service_object) in pending_services {
            let mut info = service::Info::registrable(
                service_name.clone(),
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
        }

        let services = Arc::clone(&self.services);
        let service_directory = self.service_directory.clone();
        let task = async move {
            // Execute server, which polls from incoming connections and session connections, which
            // receive messages and dispatch to messaging handlers and objects.
            let mut server_task = pin!(server_task.fuse());
            let mut session_connections = pin!(session_connections.fuse());
            loop {
                select! {
                    () = &mut server_task, if !server_task.is_terminated() => {
                        // server is terminated
                    }
                    () = &mut session_connections, if !session_connections.is_terminated() => {
                        // sessions are stopped
                    }
                    // Also update services info to the service directory when the server endpoints change.
                    Ok(()) = server_endpoints.changed() => {
                        let endpoints = server_endpoints.borrow_and_update().values().flatten().map(|&address| address.into()).collect::<Vec<_>>();
                        for service_info in services.lock().await.info_mut() {
                            service_info.endpoints.clone_from(&endpoints);
                            if let Err(_err) = service_directory.update_service_info(&*service_info).await {
                                 // TODO: log the failure
                            }
                        }
                    }
                }
            }
        };
        Ok(task)
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
        let object = object::Client::connect(
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
