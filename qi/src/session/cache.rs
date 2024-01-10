use super::{authentication, connect, Client, Service, WeakClient};
use crate::{channel, node, session, Error};
use futures::{SinkExt, StreamExt, TryStreamExt};
use qi_messaging as messaging;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::RwLock,
};

#[derive(Clone, Debug)]
pub(crate) struct Cache {
    cache: Arc<RwLock<HashMap<String, WeakClient>>>,
}

impl Cache {
    pub(crate) fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) async fn get(
        &self,
        config: node::Config,
        service_name: &str,
        messaging_service: Arc<node::Server>,
    ) -> Result<Client, Error> {
        for address in config.addresses {
            match address {
                session::Address::Relative { service } => {
                    if let Some(session) = self.cached_service_session(&service).await {
                        return Ok(session);
                    }
                }
                session::Address::Node(node::Address::Tcp {
                    host,
                    port,
                    ssl: None,
                }) => {
                    let (read, write) = TcpStream::connect((host, port)).await?.into_split();
                    if let Ok(session) = self
                        .connect_service_session_on_rw(
                            read,
                            write,
                            service_name.to_owned(),
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

    fn connect_service_session_on_rw<R, W, Svc>(
        &self,
        read: R,
        write: W,
        service_name: String,
        messaging_service: Svc,
        credentials: authentication::Parameters,
    ) -> impl Future<Output = Result<Client, Error>>
    where
        R: AsyncRead + Send + 'static,
        W: AsyncWrite + Send + 'static,
        Svc: qi_messaging::Service + Send + 'static,
    {
        let (messages_read, messages_write) = channel::open_on_rw(read, write);

        type BoxError = Box<dyn std::error::Error + Send + Sync>;
        let messages_read = messages_read.fuse().err_into::<BoxError>();
        let messages_write = messages_write.sink_err_into();

        let capabilities = Arc::new(RwLock::new(None));
        let session_service = Service::new(
            authentication::PermissiveAuthenticator,
            Arc::clone(&capabilities),
            messaging_service,
        );
        let (endpoint, client) = messaging::endpoint(messages_read, session_service);

        let session = connect(client, credentials, capabilities);
        let connection = endpoint.into_messages_stream().forward(messages_write);
        let cache = Arc::clone(&self.cache);
        tokio::spawn(async move {
            let _res = connection.await;
            cache.write().await.remove(&service_name);
        });
        session
    }

    async fn cached_service_session(&self, name: &str) -> Option<Client> {
        self.cache
            .read()
            .await
            .get(name)
            .and_then(|weak| weak.upgrade())
    }
}
