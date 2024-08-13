mod server;
mod service_map;
mod session_factory;

use self::service_map::{PendingServiceMap, ServiceMap};
use crate::{
    object::{self, Object},
    os::MachineId,
    service::{self, Info},
    service_directory::{self, ServiceDirectory},
    session::{
        self,
        authentication::{self, Authenticator, PermissiveAuthenticator},
    },
    Address,
};
use futures::{future::FusedFuture, FutureExt};
use session_factory::SessionFactory;
use std::{future::Future, pin::pin, sync::Arc};
use tokio::{select, sync::Mutex};

#[derive(Default, Debug)]
pub struct Builder<Auth, Method> {
    uid: Uid,
    authenticator: Auth,
    bind_addresses: Vec<Address>,
    pending_services: PendingServiceMap,
    method: Method,
}

impl Builder<PermissiveAuthenticator, UnsetSpaceMethod> {
    pub fn new() -> Self {
        Builder::default()
    }
}

impl<Auth, M> Builder<Auth, M>
where
    Auth: Authenticator + Send + Sync + Clone + 'static,
{
    pub fn with_authenticator<A>(self, authenticator: A) -> Builder<A, M> {
        Builder {
            authenticator,
            uid: self.uid,
            bind_addresses: self.bind_addresses,
            pending_services: self.pending_services,
            method: self.method,
        }
    }

    pub fn add_service<Name, O>(mut self, name: Name, object: O) -> Self
    where
        Name: ToString,
        O: Object + Send + Sync + 'static,
    {
        self.pending_services
            .add(name.to_string(), Box::new(object));
        self
    }

    /// Bind the node to an address, accepting incoming connections on an
    /// endpoint at this address.
    pub fn bind(mut self, address: Address) -> Self {
        self.bind_addresses.push(address);
        self
    }

    /// Attaches the node to the space hosted at the given address.
    pub fn connect_to_space(
        self,
        address: Address,
        credentials: Option<authentication::Parameters<'_>>,
    ) -> Builder<Auth, SetSpaceMethod<'_>> {
        Builder {
            authenticator: self.authenticator,
            uid: self.uid,
            bind_addresses: self.bind_addresses,
            pending_services: self.pending_services,
            method: SetSpaceMethod(SpaceMethod::ConnectToSpace(
                address,
                credentials.unwrap_or_default(),
            )),
        }
    }

    /// Host a new space on this node.
    pub fn host_space<A>(self) -> Builder<Auth, SetSpaceMethod<'static>> {
        Builder {
            authenticator: self.authenticator,
            uid: self.uid,
            bind_addresses: self.bind_addresses,
            pending_services: self.pending_services,
            method: SetSpaceMethod(SpaceMethod::HostSpace),
        }
    }
}

impl<'a, Auth> Builder<Auth, SetSpaceMethod<'a>>
where
    Auth: Authenticator + Send + Sync + Clone + 'static,
{
    pub async fn start(self) -> Result<(Node, impl Future<Output = ()>)> {
        // TODO: Task a Spawn impl to spawn futures on an executor.
        let services = Arc::default();
        let handler = service_map::MessagingHandler::new(Arc::clone(&services));
        let (mut server_endpoints, server_task) =
            server::create(handler.clone(), self.authenticator, self.bind_addresses);
        let (session_factory, session_connections) = SessionFactory::new(handler);

        let service_directory: Arc<dyn ServiceDirectory + Send + Sync> = match self.method.0 {
            SpaceMethod::ConnectToSpace(address, credentials) => {
                let session = session_factory
                    .establish(
                        service_directory::SERVICE_NAME,
                        [address.into()].iter(),
                        credentials,
                    )
                    .await?;
                Arc::new(service_directory::Client::new(session))
            }
            SpaceMethod::HostSpace => {
                todo!()
            }
        };

        let node = Node {
            services: Arc::clone(&services),
            session_factory,
            service_directory: Arc::clone(&service_directory),
        };

        // Register each service to the directory, and mark them as ready.
        for (service_name, service_object) in self.pending_services {
            let mut info = service::Info::registrable(
                service_name.clone(),
                self.uid.clone(),
                service_object.uid(),
            );
            // Registering the service to the directory gets us a service ID, that we can use to
            // update the local service info. With it, we can also index the service to the
            // messaging handler so that it can start treating requests for that service.
            // Consequently, we can notify the service directory of the readiness of the service.
            let service_id = service_directory.register_service(&info).await?;
            info.id = service_id;
            services
                .lock()
                .await
                .insert(service_name, info, service_object);
            service_directory.service_ready(service_id).await?;
        }

        let task = async move {
            // Execute server, which polls from incoming connections and session connections, which
            // receive messages and dispatch to messaging handlers and objects.
            let mut server_task = server_task.fuse();
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
        Ok((node, task))
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct UnsetSpaceMethod;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetSpaceMethod<'a>(SpaceMethod<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum SpaceMethod<'a> {
    ConnectToSpace(Address, authentication::Parameters<'a>),
    HostSpace,
}

pub struct Node {
    services: Arc<Mutex<ServiceMap>>,
    session_factory: SessionFactory<service_map::MessagingHandler>,
    service_directory: Arc<dyn ServiceDirectory + Send + Sync>,
}

impl Node {
    pub async fn service(&self, name: &str) -> Result<impl Object> {
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
        Ok(object)
    }

    pub fn service_directory(&self) -> &dyn ServiceDirectory {
        self.service_directory.as_ref()
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttachedNode")
            .field("services", &self.services)
            .field("session_factory", &self.session_factory)
            .finish()
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

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
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
