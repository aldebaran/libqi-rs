use super::{authentication, reference, Address, Reference, Session};
use crate::{messaging::message, Error};
use bytes::Bytes;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;

/// A session cache that handles session references and keeps track of existing
/// sessions to a space services.
///
/// It creates new sessions that are associated with services and register them
/// for further retrieval, enabling usage of service session references.
#[derive(Clone, Debug)]
pub(crate) struct Cache<Svc> {
    /// The service used to open messaging endpoints and receive
    /// messages from new session channels.
    service: Svc,

    /// The list of existing sessions with the associated service name.
    sessions: Arc<Mutex<HashMap<String, Weak<Session>>>>,
}

impl<Svc> Cache<Svc> {
    pub(crate) fn new(service: Svc) -> Self {
        Self {
            service,
            sessions: Arc::default(),
        }
    }

    async fn existing_service_session(&self, name: &str) -> Option<Arc<Session>> {
        self.sessions
            .lock()
            .await
            .get(name)
            .and_then(|weak| weak.upgrade())
    }
}

impl<Svc> Cache<Svc>
where
    Svc: tower::Service<(message::Address, Bytes)> + Clone + 'static,
{
    /// Gets a session to the given references, using the service name to store
    /// any created session for further retrieval.
    pub(crate) async fn get<R>(
        &self,
        service_name: &str,
        references: R,
        credentials: authentication::Parameters,
    ) -> Result<Arc<Session>, Error>
    where
        R: IntoIterator<Item = Reference>,
    {
        for reference in references {
            match reference.0 {
                reference::Inner::Service(ref service) => {
                    if let Some(session) = self.existing_service_session(service).await {
                        return Ok(session);
                    }
                }
                reference::Inner::Endpoint(address) => {
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
    ) -> Result<Arc<Session>, Error> {
        let (session, connection) =
            Session::connect(address, credentials, self.service.clone()).await?;
        let session = Arc::new(session);
        let service_name = service_name.to_owned();
        self.sessions
            .lock()
            .await
            .insert(service_name.to_owned().clone(), Arc::downgrade(&session));
        let sessions = Arc::clone(&self.sessions);
        tokio::spawn(async move {
            let _res = connection.await;
            sessions.lock().await.remove(&service_name);
        });
        Ok(session)
    }
}
