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
use tower::{Service, ServiceExt};

#[derive(Clone, Debug)]
pub(crate) struct Session {
    uid: Uid,
    capabilities: watch::Receiver<Option<CapabilitiesMap<'static>>>,
    client: messaging::Client<BinaryValue, BinaryValue>,
}

impl Session {
    pub(crate) async fn connect<CallHandler, OnewaySink>(
        address: messaging::Address,
        credentials: authentication::Parameters<'_>,
        call_handler: CallHandler,
        oneway_sink: OnewaySink,
    ) -> Result<(Self, impl Future<Output = Result<()>>)>
    where
        CallHandler:
            tower::Service<(message::Address, BinaryValue), Error = Error, Response = BinaryValue>,
        OnewaySink: Sink<(message::Address, message::OnewayRequest<BinaryValue>), Error = Error>,
    {
        let Control {
            controller,
            capabilities,
            call_handler,
            oneway_sink,
            ..
        } = control::make(call_handler, oneway_sink, PermissiveAuthenticator, true);
        let (messages_stream, messages_sink) = messaging::channel::connect(address).await?;
        let (mut client, connection) =
            messaging::endpoint::start(messages_stream, messages_sink, call_handler, oneway_sink);
        controller
            .authenticate_to_remote(&mut client, credentials)
            .await?;
        Ok((
            Session {
                uid: Uid::new(),
                capabilities,
                client,
            },
            connection.map_err(Into::into),
        ))
    }

    pub(crate) async fn bind_server<Auth, CallHandler, OnewaySink>(
        address: messaging::Address,
        authenticator: Auth,
        call_handler: CallHandler,
        oneway_sink: OnewaySink,
    ) -> Result<(impl Future<Output = ()>, Vec<messaging::Address>)>
    where
        Auth: Authenticator + Clone + Send + Sync + 'static,
        CallHandler: tower::Service<(message::Address, BinaryValue), Error = Error, Response = BinaryValue>
            + Clone,
        OnewaySink:
            Sink<(message::Address, message::OnewayRequest<BinaryValue>), Error = Error> + Clone,
    {
        let (clients, endpoints) = messaging::channel::serve(address).await?;
        let server = async move {
            let mut clients = pin!(clients.fuse());
            let mut sessions = FuturesUnordered::new();
            loop {
                select! {
                    Some((messages_stream, messages_sink, _address)) = clients.next(), if !clients.is_terminated() => {
                        sessions.push(Self::serve(messages_stream, messages_sink, authenticator.clone(), call_handler.clone(), oneway_sink.clone()));
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

    async fn serve<Auth, MsgStream, MsgSink, CallHandler, OnewaySink>(
        messages_stream: MsgStream,
        messages_sink: MsgSink,
        authenticator: Auth,
        call_handler: CallHandler,
        oneway_sink: OnewaySink,
    ) where
        MsgStream:
            Stream<Item = std::result::Result<messaging::Message<BinaryValue>, messaging::Error>>,
        MsgSink: Sink<messaging::Message<BinaryValue>, Error = messaging::Error>,
        Auth: Authenticator + Send + Sync + 'static,
        CallHandler:
            tower::Service<(message::Address, BinaryValue), Error = Error, Response = BinaryValue>,
        OnewaySink: Sink<(message::Address, message::OnewayRequest<BinaryValue>), Error = Error>,
    {
        let Control {
            capabilities,
            mut remote_authorized,
            call_handler,
            oneway_sink,
            ..
        } = control::make(call_handler, oneway_sink, authenticator, true);
        let (client, connection) =
            messaging::endpoint::start(messages_stream, messages_sink, call_handler, oneway_sink);
        let mut _session = None;
        let mut connection = pin!(connection);
        loop {
            select! {
                authorized = remote_authorized.changed() => {
                    match authorized {
                        Ok(()) => if *remote_authorized.borrow_and_update() {
                            _session = Some(Self {
                                uid: Uid::new(),
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
            .clone()
            .ready_oneshot()
            .await?
            .call((address, BinaryValue::serialize(&value)?))
            .await?
            .deserialize_value(return_type)
            .map(|value| value.into_owned())
    }

    pub(crate) fn uid(&self) -> Uid {
        self.uid.clone()
    }

    pub(crate) fn downgrade(&self) -> WeakSession {
        WeakSession {
            uid: self.uid.clone(),
            capabilities: self.capabilities.clone(),
            client: self.client.downgrade(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WeakSession {
    uid: Uid,
    capabilities: watch::Receiver<Option<CapabilitiesMap<'static>>>,
    client: messaging::WeakClient<BinaryValue, BinaryValue>,
}

impl WeakSession {
    pub(crate) fn upgrade(&self) -> Option<Session> {
        self.client.upgrade().map(|client| Session {
            uid: self.uid.clone(),
            capabilities: self.capabilities.clone(),
            client,
        })
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "crate::value", transparent)]
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
