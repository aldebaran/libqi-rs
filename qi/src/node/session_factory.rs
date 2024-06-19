use crate::{
    messaging,
    session::{self, authentication, Session, WeakSession},
    value::BinaryValue,
    Error,
};
use futures::{future::BoxFuture, stream::FuturesUnordered, FutureExt, StreamExt};
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::{
    select,
    sync::{mpsc, Mutex},
};

/// A session cache that handles session references and keeps track of existing
/// sessions to a space services.
///
/// It creates new sessions that are associated with services and register them
/// for further retrieval, enabling usage of service session references.
#[derive(Debug)]
pub(super) struct SessionFactory<Handler> {
    handler: Handler,

    connections: mpsc::Sender<BoxFuture<'static, String>>,

    /// The list of existing sessions with the associated service name.
    sessions: Arc<Mutex<HashMap<String, WeakSession>>>,
}

impl<Handler> SessionFactory<Handler> {
    pub(super) fn new(handler: Handler) -> (Self, impl Future<Output = ()>) {
        let sessions = Default::default();
        let (connections_sender, mut connections_receiver) = mpsc::channel(1);
        (
            Self {
                handler,
                sessions: Arc::clone(&sessions),
                connections: connections_sender,
            },
            async move {
                let mut connections = FuturesUnordered::new();
                loop {
                    select! {
                        Some(connection) = connections_receiver.recv() => {
                            connections.push(connection);
                        }
                        Some(name) = connections.next() => {
                            sessions.lock().await.remove(&name);
                        }
                        else => break,
                    }
                }
            },
        )
    }

    async fn existing_service_session(&self, name: &str) -> Option<Session> {
        let mut sessions = self.sessions.lock().await;
        match sessions.get(name) {
            Some(weak) => {
                let session = weak.upgrade();
                if session.is_none() {
                    sessions.remove(name);
                }
                session
            }
            None => None,
        }
    }
}

impl<Handler> SessionFactory<Handler>
where
    Handler: messaging::Handler<BinaryValue, Reply = BinaryValue, Error = Error>
        + Clone
        + Send
        + Sync
        + 'static,
{
    /// Gets a session to the given references, using the service name to store
    /// any created session for further retrieval.
    pub(super) async fn establish<'r, R>(
        &self,
        service_name: &str,
        references: R,
        credentials: authentication::Parameters<'_>,
    ) -> Result<Session, Error>
    where
        R: IntoIterator<Item = &'r session::Reference>,
    {
        for reference in references {
            match reference.kind() {
                session::reference::Kind::Service(service) => {
                    if let Some(session) = self.existing_service_session(service).await {
                        return Ok(session);
                    }
                }
                session::reference::Kind::Endpoint(address) => {
                    if let Ok(session) = self
                        .connect_service_session(service_name, *address, credentials.clone())
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
        address: messaging::Address,
        credentials: authentication::Parameters<'_>,
    ) -> Result<Session, Error> {
        let (session, connection) =
            Session::connect(address, credentials, self.handler.clone()).await?;
        let service_name = service_name.to_owned();
        self.sessions
            .lock()
            .await
            .insert(service_name.clone(), session.downgrade());
        self.connections
            .send(
                async move {
                    let _res = connection.await;
                    service_name
                }
                .boxed(),
            )
            .await
            .map_err(|_err| Error::Messaging(messaging::Error::Canceled))?;
        Ok(session)
    }
}
