pub mod authentication;
mod capabilities;
mod control;
pub(crate) mod reference;

pub use self::reference::Reference;
use crate::{
    messaging::{self, message, CapabilitiesMap},
    session::authentication::{Authenticator, PermissiveAuthenticator},
    value, BinaryValue, Error, Result,
};
use control::Control;
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Sink, Stream, StreamExt, TryFutureExt,
};
use qi_messaging::BodyBuf;
use std::{future::Future, pin::pin};
use tokio::{select, sync::watch};

#[derive(Clone, Debug)]
pub(crate) struct Session {
    capabilities: watch::Receiver<Option<CapabilitiesMap<'static>>>,
    client: messaging::Client<BinaryValue, BinaryValue>,
}

impl Session {
    pub(crate) async fn connect<Handler>(
        address: messaging::Address,
        credentials: authentication::Parameters<'_>,
        handler: Handler,
    ) -> Result<(Self, impl Future<Output = Result<()>>)>
    where
        Handler: messaging::Handler<BinaryValue, Error = Error, Reply = BinaryValue> + Sync,
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
            .authenticate_to_remote(&mut client, credentials)
            .await?;
        Ok((
            Session {
                capabilities,
                client,
            },
            connection.map_err(Into::into),
        ))
    }

    pub(crate) async fn bind_server<Auth, Handler>(
        address: messaging::Address,
        authenticator: Auth,
        handler: Handler,
    ) -> Result<(impl Future<Output = ()>, Vec<messaging::Address>)>
    where
        Auth: Authenticator + Clone + Send + Sync + 'static,
        Handler: messaging::Handler<BinaryValue, Error = Error, Reply = BinaryValue> + Sync + Clone,
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

    async fn serve<Auth, MsgStream, MsgSink, Handler>(
        messages_stream: MsgStream,
        messages_sink: MsgSink,
        authenticator: Auth,
        handler: Handler,
    ) where
        MsgStream:
            Stream<Item = std::result::Result<messaging::Message<BinaryValue>, messaging::Error>>,
        MsgSink: Sink<messaging::Message<BinaryValue>, Error = messaging::Error>,
        Auth: Authenticator + Send + Sync + 'static,
        Handler: messaging::Handler<BinaryValue, Error = Error, Reply = BinaryValue> + Sync,
    {
        let Control {
            capabilities,
            mut remote_authorized,
            handler,
            ..
        } = control::make(handler, authenticator, true);
        let (client, connection) =
            messaging::endpoint::start(messages_stream, messages_sink, handler);
        let mut _session = None;
        let mut connection = pin!(connection);
        loop {
            select! {
                authorized = remote_authorized.changed() => {
                    match authorized {
                        Ok(()) => if *remote_authorized.borrow_and_update() {
                            _session = Some(Self {
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
    ) -> Result<value::Value<'static>> {
        self.client
            .call(address, BinaryValue::serialize(&value)?)
            .await?
            .deserialize_value_of_type(return_type)
            .map(|value| value.into_owned())
    }

    pub(crate) async fn oneway(
        &self,
        address: message::Address,
        request: message::Oneway<value::Value<'_>>,
    ) -> Result<()> {
        let request = request.try_map(|value| BinaryValue::serialize(&value))?;
        self.client.oneway(address, request).await?;
        Ok(())
    }

    pub(crate) fn downgrade(&self) -> WeakSession {
        WeakSession {
            capabilities: self.capabilities.clone(),
            client: self.client.downgrade(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WeakSession {
    capabilities: watch::Receiver<Option<CapabilitiesMap<'static>>>,
    client: messaging::WeakClient<BinaryValue, BinaryValue>,
}

impl WeakSession {
    pub(crate) fn upgrade(&self) -> Option<Session> {
        self.client.upgrade().map(|client| Session {
            capabilities: self.capabilities.clone(),
            client,
        })
    }
}
