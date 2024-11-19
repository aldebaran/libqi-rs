pub mod authentication;
mod capabilities;
pub mod control;
mod map;
mod target;

pub(crate) use self::map::Map;
pub use self::target::Target;
use crate::{
    error::{Error, FormatError},
    messaging::{self, message},
    session::authentication::{Authenticator, PermissiveAuthenticator},
    value::{self, KeyDynValueMap},
};
use control::Control;
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Sink, Stream, StreamExt,
};
use std::{future::Future, pin::pin};
use tokio::{select, sync::watch};

pub struct Session<Body> {
    capabilities: watch::Receiver<Option<KeyDynValueMap>>,
    client: messaging::Client<Body>,
}

impl<Body> Session<Body>
where
    Body: messaging::Body + Send,
    Body::Error: Send + Sync + 'static,
{
    pub(crate) async fn connect<Handler>(
        address: messaging::Address,
        credentials: KeyDynValueMap,
        handler: Handler,
    ) -> Result<(Self, impl Future<Output = Result<(), messaging::Error>>), Error>
    where
        Handler: messaging::Handler<Body> + Sync,
        Handler::Error: std::error::Error + Send + Sync + 'static,
    {
        let Control {
            controller,
            capabilities,
            handler,
            ..
        } = control::make(handler, PermissiveAuthenticator, true);
        let (messages_stream, messages_sink) = messaging::channel::connect(address).await?;
        let (mut client, connection) =
            messaging::endpoint::start(messages_stream, messages_sink, handler);
        controller
            .authenticate_to_server(&mut client, credentials)
            .await?;
        Ok((
            Session {
                capabilities,
                client,
            },
            connection,
        ))
    }

    pub(crate) async fn bind_server<Auth, Handler>(
        address: messaging::Address,
        authenticator: Auth,
        handler: Handler,
    ) -> Result<(impl Future<Output = ()>, Vec<messaging::Address>), messaging::Error>
    where
        Auth: Authenticator + Clone + Send + Sync + 'static,
        Handler: messaging::Handler<Body> + Sync + Clone,
        Handler::Error: std::error::Error + Send + Sync + 'static,
    {
        let (clients, endpoints) = messaging::channel::serve(address).await?;
        let server = async move {
            let mut clients = pin!(clients.fuse());
            let mut sessions = FuturesUnordered::new();
            loop {
                select! {
                    Some((messages_stream, messages_sink, _address)) = clients.next(), if !clients.is_terminated() => {
                        sessions.push(Self::serve(messages_stream, messages_sink, authenticator.clone(), handler.clone()));
                    }
                    _res = sessions.next(), if !sessions.is_terminated() => {
                        // nothing
                    }
                    else => {
                        break
                    }
                }
            }
        };
        Ok((server, endpoints))
    }

    pub async fn serve<Auth, MsgStream, MsgSink, Handler>(
        messages_stream: MsgStream,
        messages_sink: MsgSink,
        authenticator: Auth,
        handler: Handler,
    ) where
        MsgStream: Stream<Item = std::result::Result<messaging::Message<Body>, messaging::Error>>,
        MsgSink: Sink<messaging::Message<Body>, Error = messaging::Error>,
        Auth: Authenticator + Send + Sync + 'static,
        Handler: messaging::Handler<Body> + Sync,
        Handler::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let Control {
            capabilities,
            mut remote_authorized,
            handler,
            ..
        } = control::make(handler, authenticator, true);
        let (client, connection) =
            messaging::endpoint::start(messages_stream, messages_sink, handler);
        let client = client.downgrade();
        let mut _session = None;
        let mut connection = pin!(connection);
        loop {
            select! {
                authorized = remote_authorized.changed() => {
                    match authorized {
                        Ok(()) => if *remote_authorized.borrow_and_update() {
                            _session = Some(WeakSession {
                                capabilities: capabilities.clone(),
                                client: client.clone(),
                            })
                        } else {
                            _session = None;
                        }
                        Err(_err) => {
                            // Control has been dropped, stop serving the connection.
                            break;
                        }
                    }
                }
                _res = &mut connection => {
                    break;
                }
            }
        }
    }

    pub(crate) async fn call(
        &self,
        address: message::Address,
        value: value::Value<'_>,
        return_type: Option<&value::Type>,
    ) -> Result<value::Value<'static>, Error> {
        let args = Body::serialize(&value).map_err(FormatError::ArgumentsSerialization)?;
        Ok(self
            .client
            .call(address, args)
            .await?
            .deserialize_seed(value::de::ValueType(return_type))
            .map_err(FormatError::MethodReturnValueDeserialization)?
            .into_owned())
    }

    pub(crate) async fn fire_and_forget(
        &self,
        address: message::Address,
        request: message::FireAndForget<value::Value<'_>>,
    ) -> Result<(), Error> {
        let request = request
            .try_map(|value| Body::serialize(&value))
            .map_err(FormatError::ArgumentsSerialization)?;
        self.client.fire_and_forget(address, request).await?;
        Ok(())
    }

    pub(crate) fn downgrade(&self) -> WeakSession<Body> {
        WeakSession {
            capabilities: self.capabilities.clone(),
            client: self.client.downgrade(),
        }
    }
}

impl<Body> Clone for Session<Body> {
    fn clone(&self) -> Self {
        Self {
            capabilities: self.capabilities.clone(),
            client: self.client.clone(),
        }
    }
}

impl<Body> std::fmt::Debug for Session<Body> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("capabilities", &self.capabilities)
            .field("client", &self.client)
            .finish()
    }
}

pub(crate) struct WeakSession<Body> {
    capabilities: watch::Receiver<Option<KeyDynValueMap>>,
    client: messaging::WeakClient<Body>,
}

impl<Body> WeakSession<Body> {
    pub(crate) fn upgrade(&self) -> Option<Session<Body>> {
        self.client.upgrade().map(|client| Session {
            capabilities: self.capabilities.clone(),
            client,
        })
    }
}

impl<Body> Clone for WeakSession<Body> {
    fn clone(&self) -> Self {
        Self {
            capabilities: self.capabilities.clone(),
            client: self.client.clone(),
        }
    }
}

impl<Body> std::fmt::Debug for WeakSession<Body> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakSession")
            .field("capabilities", &self.capabilities)
            .field("client", &self.client)
            .finish()
    }
}
