use crate::{
    channel,
    session::{self, authentication, Config},
    Address, ConnectionError, Error,
};
use futures::{
    future::BoxFuture, stream::FusedStream, FutureExt, SinkExt, StreamExt, TryStreamExt,
};
use qi_messaging as messaging;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::{mpsc, RwLock},
};

use super::Server;

#[derive(Clone, Debug)]
pub(super) struct Sessions {
    connections: mpsc::Sender<SessionConnection>,
    cache: Arc<RwLock<HashMap<String, session::WeakClient>>>,
}

impl Sessions {
    pub(super) fn new() -> (Self, impl Future<Output = ()>) {
        let (connections_sender, mut connections_receiver) = mpsc::channel(1);
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let this = Self {
            connections: connections_sender,
            cache: Arc::clone(&cache),
        };
        let task = async move {
            let mut connections = futures::stream::FuturesUnordered::new();
            loop {
                tokio::select! {
                    Some(connection) = connections_receiver.recv() => {
                        connections.push(connection)
                    }
                    Some((name, _res)) = connections.next(), if !connections.is_terminated() => {
                        // TODO: trace/log connection termination
                        cache.write().await.remove(&name);
                    }
                    else => {
                        break;
                    }
                }
            }
        };
        (this, task)
    }

    pub(super) async fn get(
        &self,
        config: Config,
        service_name: &str,
        messaging_service: Arc<Server>,
    ) -> Result<session::Client, Error> {
        for address in config.addresses {
            match address {
                Address::Relative { service } => {
                    if let Some(session) = self.cached_service_session(&service).await {
                        return Ok(session);
                    }
                }
                Address::Tcp {
                    host,
                    port,
                    ssl: None,
                } => {
                    let (read, write) = TcpStream::connect((host, port)).await?.into_split();
                    if let Ok(session) = self
                        .connect_session_to_service_node(
                            read,
                            write,
                            service_name,
                            Arc::clone(&messaging_service),
                            config.credentials.clone(),
                        )
                        .await
                    {
                        return Ok(session);
                    }
                }
                _ => todo!(),
            }
        }
        Err(Error::NoReachableEndpoint)
    }

    fn connect_session_to_service_node<R, W>(
        &self,
        read: R,
        write: W,
        service_name: &str,
        messaging_service: Arc<Server>,
        credentials: authentication::Parameters,
    ) -> impl Future<Output = Result<session::Client, Error>>
    where
        R: AsyncRead + Send + 'static,
        W: AsyncWrite + Send + 'static,
    {
        let (messages_read, messages_write) = channel::open_on_rw(read, write);

        let messages_read = messages_read.fuse().map_err(ConnectionError::Decode);
        let messages_write = messages_write.sink_map_err(ConnectionError::Encode);

        let capabilities = Arc::new(RwLock::new(None));
        let service = session::Service::new(
            authentication::PermissiveAuthenticator,
            Arc::clone(&capabilities),
            messaging_service,
        );
        let (endpoint, client) = messaging::endpoint(messages_read, service);

        let session = session::connect(client, credentials, capabilities);
        let connection = endpoint.into_messages_stream().forward(messages_write);
        let connection = SessionConnection::new(connection.boxed(), service_name.to_owned());
        let connections = self.connections.clone();
        async move {
            connections.send(connection).await?;
            session.await
        }
    }

    async fn cached_service_session(&self, name: &str) -> Option<session::Client> {
        self.cache
            .read()
            .await
            .get(name)
            .and_then(|weak| weak.upgrade())
    }
}

struct SessionConnection {
    future: BoxFuture<'static, Result<(), ConnectionError>>,
    service_name: Option<String>,
}

impl SessionConnection {
    fn new(future: BoxFuture<'static, Result<(), ConnectionError>>, service_name: String) -> Self {
        Self {
            future,
            service_name: Some(service_name),
        }
    }
}

impl Future for SessionConnection {
    type Output = (String, Result<(), ConnectionError>);

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let res = std::task::ready!(std::pin::Pin::new(&mut self.future).poll(cx));
        std::task::Poll::Ready((self.service_name.take().unwrap(), res))
    }
}
