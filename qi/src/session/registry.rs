use super::{authentication, connect, Client, Reference, Service, WeakClient};
use crate::{channel, Address, Error};
use futures::{SinkExt, StreamExt, TryStreamExt};
use qi_messaging as messaging;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// A session registry keeps track of existing sessions to a space services.
///
/// It creates new sessions to services and register them for further retrieval,
/// and enables handling of service session references.
#[derive(Clone, Debug)]
pub(crate) struct Registry<Svc> {
    /// The messaging service used to open messaging endpoints and receive
    /// messages from new session channels.
    messaging_service: Svc,

    /// The list of existing sessions with the associated service name.
    clients: Arc<RwLock<HashMap<String, WeakClient>>>,
}

impl<Svc> Registry<Svc> {
    pub(crate) fn new(messaging_service: Svc) -> Self {
        Self {
            messaging_service,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn existing_service_session(&self, name: &str) -> Option<Client> {
        self.clients
            .read()
            .await
            .get(name)
            .and_then(|weak| weak.upgrade())
    }
}

impl<Svc> Registry<Svc>
where
    Svc: qi_messaging::Service + Clone + Send + 'static,
{
    /// Gets a session to the given references, using the service name to store
    /// any created session for further retrieval.
    pub(crate) async fn get(
        &self,
        service_name: &str,
        references: Vec<Reference>,
        credentials: authentication::Parameters,
    ) -> Result<Client, Error> {
        for reference in references {
            match reference {
                Reference::Service(service) => {
                    if let Some(session) = self.existing_service_session(&service).await {
                        return Ok(session);
                    }
                }
                Reference::Endpoint(address) => {
                    if let Ok(session) = self
                        .connect_service_session(service_name, address, credentials.clone())
                        .await
                    {
                        return Ok(session);
                    }
                }
            }
        }
        Err(Error::NoReachableEndpoint)
    }

    /// Opens a new channel to the address and connects a session client over
    /// it. The service name is used to register the session for this service,
    /// so that it may be reused for future service relative session references.
    async fn connect_service_session(
        &self,
        service_name: &str,
        address: Address,
        credentials: authentication::Parameters,
    ) -> Result<Client, Error> {
        let (messages_read, messages_write) = channel::open(address).await?;

        type BoxError = Box<dyn std::error::Error + Send + Sync>;
        let messages_read = messages_read.fuse().err_into::<BoxError>();
        let messages_write = messages_write.sink_err_into();

        let capabilities = Arc::new(RwLock::new(None));
        let session_service = Service::new(
            authentication::PermissiveAuthenticator,
            Arc::clone(&capabilities),
            self.messaging_service.clone(),
        );
        let (endpoint, client) = messaging::endpoint(messages_read, session_service);
        let connection = endpoint.into_messages_stream().forward(messages_write);
        let clients = Arc::clone(&self.clients);
        let service_name = service_name.to_owned();
        tokio::spawn(async move {
            let _res = connection.await;
            clients.write().await.remove(&service_name);
        });

        connect(client, credentials, capabilities).await
    }
}
