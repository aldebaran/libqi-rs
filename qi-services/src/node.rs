use crate::{sd, service::ServiceInfo, session, space, Address, ConnectionError, Error};
use bytes::Bytes;
use futures::{
    future::BoxFuture,
    stream::{FusedStream, FuturesUnordered},
    FutureExt, StreamExt,
};
use once_cell::sync::Lazy;
use qi_messaging as messaging;
use qi_value::Value;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};
use tokio::{
    select,
    sync::{mpsc, watch},
};
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug)]
pub struct Node {
    service: Arc<Service>,
    connections_sender: mpsc::Sender<SessionConnection>,
    service_sessions: watch::Receiver<HashMap<String, session::Client>>,
}

impl Node {
    pub fn open() -> (Self, impl Future<Output = ()>) {
        let (connections_sender, connections_receiver) = mpsc::channel(1);
        let (service_sessions_sender, service_sessions_receiver) = watch::channel(HashMap::new());
        let node = Self {
            service: Arc::new(Service),
            connections_sender,
            service_sessions: service_sessions_receiver,
        };
        let task = task(connections_receiver, service_sessions_sender);
        (node, task)
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn connect_to_space<'this, E>(
        &'this self,
        endpoints: E,
        authentication_parameters: Option<HashMap<String, Value<'_>>>,
    ) -> Result<space::Client<'this>, Error>
    where
        E: IntoIterator,
        E::Item: TryInto<Address>,
    {
        let session = self
            .session_to_service_node(
                endpoints,
                sd::SERVICE_NAME.to_owned(),
                authentication_parameters,
            )
            .await?;
        Ok(space::Client::new(self, session))
    }

    pub(crate) async fn session_to_service_node<E>(
        &self,
        endpoints: E,
        service_name: String,
        authentication_parameters: Option<HashMap<String, Value<'_>>>,
    ) -> Result<session::Client, Error>
    where
        E: IntoIterator,
        E::Item: TryInto<Address>,
    {
        for endpoint in endpoints {
            if let Ok(endpoint) = endpoint.try_into() {
                match endpoint {
                    Address::Qi { service } => {
                        if let Some(session) = self.service_sessions.borrow().get(&service) {
                            return Ok(session.clone());
                        }
                    }
                    endpoint => {
                        if let Ok(session) = self
                            .start_session(
                                endpoint,
                                service_name.clone(),
                                authentication_parameters.clone(),
                            )
                            .await
                        {
                            return Ok(session.clone());
                        }
                    }
                }
            }
        }
        Err(Error::NoReachableEndpoint)
    }

    pub(crate) async fn start_session(
        &self,
        endpoint: Address,
        service_name: String,
        authentication_parameters: Option<HashMap<String, Value<'_>>>,
    ) -> Result<session::Client, Error> {
        let (session, connection) = session::connect(
            endpoint,
            Arc::clone(&self.service),
            authentication_parameters,
        )
        .await?;
        self.connections_sender
            .send(SessionConnection::new(connection, service_name))
            .await
            .map_err(|_err| Error::Disconnected)?;
        session.await
    }

    pub(crate) fn sort_endpoints(&self, service: &ServiceInfo) -> Vec<Address> {
        let mut endpoints = service.endpoints.clone();
        let service_is_local = &service.machine_id == MachineId::local();
        endpoints.sort_by_cached_key(|endpoint| {
            (
                endpoint.is_relative(),
                service_is_local && endpoint.is_loopback(),
            )
        });
        endpoints
    }
}

async fn task(
    mut connections_receiver: mpsc::Receiver<SessionConnection>,
    service_sessions: watch::Sender<HashMap<String, session::Client>>,
) {
    let mut connections = FuturesUnordered::new();
    loop {
        select! {
            Some(connection) = connections_receiver.recv() => {
                connections.push(connection)
            }
            Some((name, _res)) = connections.next(), if !connections.is_terminated() => {
                // TODO: trace/log connection termination
                service_sessions.send_modify(|sessions| { sessions.remove(&name); });
            }
            else => {
                break;
            }
        }
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    qi_macros::Reflect,
    qi_macros::FromValue,
    qi_macros::IntoValue,
    qi_macros::ToValue,
)]
#[qi(transparent)]
pub struct MachineId(String);

impl MachineId {
    #[cfg(test)]
    pub(crate) fn new(id: String) -> Self {
        Self(id)
    }

    pub(crate) fn local() -> &'static Self {
        static LOCAL: Lazy<MachineId> = Lazy::new(|| {
            if let Some(id) = MachineId::from_config() {
                return id;
            }

            let mut id = None;
            if cfg!(feature = "machine-uid") {
                id = machine_uid::get().ok().map(Self);
            }
            id.unwrap_or_else(|| {
                let uuid = MachineId::generate_uuid();
                if let Some(path) = MachineId::config_path() {
                    let _res = std::fs::write(path, &uuid);
                }
                Self(uuid)
            })
        });
        &LOCAL
    }

    fn from_config() -> Option<Self> {
        std::fs::read_to_string(Self::config_path()?).ok().map(Self)
    }

    fn config_path() -> Option<std::path::PathBuf> {
        let mut dir = dirs::config_dir()?;
        dir.push("qimessaging");
        dir.push("machine_id");
        Some(dir)
    }

    fn generate_uuid() -> String {
        Uuid::new_v4().to_string()
    }
}

#[derive(Debug)]
struct Service;

impl messaging::Service for Service {
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
struct SessionConnection {
    future: BoxFuture<'static, Result<(), ConnectionError>>,
    service_name: Option<String>,
}

impl SessionConnection {
    fn new<F>(connection: F, service_name: String) -> Self
    where
        F: Future<Output = Result<(), ConnectionError>> + Send + 'static,
    {
        Self {
            future: connection.boxed(),
            service_name: Some(service_name),
        }
    }
}

impl Future for SessionConnection {
    type Output = (String, Result<(), ConnectionError>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = ready!(self.future.poll_unpin(cx));
        Poll::Ready((self.service_name.take().unwrap(), res))
    }
}
