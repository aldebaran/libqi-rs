pub mod auth;
mod capabilities;
pub(crate) mod control;
mod map;
mod target;

use self::auth::PermissiveAuthenticator;
pub(crate) use self::{auth::Authenticator, map::Map, target::Target};
use crate::{
    error::{Error, FormatError, HandlerError},
    messaging::{self, message},
    value::{self, KeyDynValueMap},
};
use control::Control;
use futures::{stream::FusedStream, Sink, StreamExt, TryStream};
use qi_messaging::Address;
use std::{net::SocketAddr, pin::pin};
use tokio::{select, sync::watch, task, time};

pub(crate) struct Session<Body> {
    capabilities: watch::Receiver<Option<KeyDynValueMap>>,
    client: messaging::Client<Body>,
}

impl<Body> Session<Body>
where
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    pub(crate) async fn connect<MsgStream, MsgSink, Handler>(
        messages_stream: MsgStream,
        messages_sink: MsgSink,
        credentials: KeyDynValueMap,
        handler: Handler,
    ) -> Result<Self, Error>
    where
        MsgStream: TryStream<Ok = messaging::Message<Body>> + Send + 'static,
        MsgStream::Error: Send,
        MsgSink: Sink<messaging::Message<Body>> + Send + 'static,
        Handler: messaging::Handler<Body, Error = HandlerError> + Send + Sync + 'static,
    {
        let Control {
            controller,
            capabilities,
            handler,
            ..
        } = control::make(handler, PermissiveAuthenticator, true);
        let (mut client, connection) =
            messaging::endpoint::start(messages_stream, messages_sink, handler);
        task::spawn(async move {
            let _res = connection.await;
        });
        controller
            .authenticate_to_server(&mut client, credentials)
            .await?;
        Ok(Session {
            capabilities,
            client,
        })
    }

    /// Binds a server of sessions to an address.
    ///
    /// Spawn a server task that:
    ///   1) spawns a session server side with the given authenticator and messaging handler each
    ///      time a client connects to the server.
    ///   2) updates a list of endpoints for this session. The list of endpoints changes if the
    ///      address targets multiple interfaces and interfaces availability changes on the system.
    ///
    /// The future terminates when the server is bound and clients can connect. The return value is a
    /// watch receiver of a pair of:
    ///   - a local address that the server is bound to.
    ///   - a list of endpoints that clients can connect to.
    ///
    /// The receiver is severed from its sender when the server is stopped.
    pub(crate) async fn server<Auth, Handler>(
        address: messaging::Address,
        authenticator: Auth,
        handler: Handler,
    ) -> Result<Server, std::io::Error>
    where
        Auth: Authenticator + Clone + Send + Sync + 'static,
        Handler: messaging::Handler<Body, Error = HandlerError> + Send + Sync + Clone + 'static,
    {
        let (clients, local_address) = messaging::channel::serve(address).await?;
        let (mut endpoints_sender, endpoints_receiver) =
            watch::channel((local_address, Vec::new()));
        let task = task::spawn(async move {
            let mut clients = pin!(clients.fuse());
            let mut update_endpoints = pin!(update_address_endpoints(
                local_address,
                &mut endpoints_sender
            ));
            // Use a join set so that when this task is dropped, all spawned client session tasks are aborted.
            let mut client_tasks = task::JoinSet::new();
            loop {
                select! {
                    Some((messages_stream, messages_sink, _address)) = clients.next(), if !clients.is_terminated() => {
                        client_tasks.spawn(Session::serve_client(
                            messages_stream,
                            messages_sink,
                            authenticator.clone(),
                            handler.clone(),
                        ));
                    }
                    () = &mut update_endpoints => {
                        // nothing, if this future terminates it means that the address was not an
                        // "ANY" IP address. The endpoints sender must not be dropped.
                    }
                    else => {
                        break;
                    }
                }
            }
        });
        Ok(Server {
            endpoints: endpoints_receiver,
            task,
        })
    }

    pub(crate) async fn serve_client<Auth, MsgStream, MsgSink, Handler>(
        messages_stream: MsgStream,
        messages_sink: MsgSink,
        authenticator: Auth,
        handler: Handler,
    ) where
        MsgStream: TryStream<Ok = messaging::Message<Body>> + Send + 'static,
        MsgStream::Error: Send,
        MsgSink: Sink<messaging::Message<Body>> + Send + 'static,
        Auth: Authenticator + Send + Sync + 'static,
        Handler: messaging::Handler<Body, Error = HandlerError> + Send + Sync + 'static,
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
        task::spawn(async move {
            let _res = connection.await;
        });

        while let Ok(()) = remote_authorized.changed().await {
            if *remote_authorized.borrow_and_update() {
                _session = Some(Session {
                    capabilities: capabilities.clone(),
                    client: client.clone(),
                })
            } else {
                _session = None;
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

#[derive(Debug)]
pub(crate) struct Server {
    endpoints: watch::Receiver<(Address, Vec<Address>)>,
    task: task::JoinHandle<()>,
}

impl Server {
    pub(crate) fn endpoints_receiver(&mut self) -> &mut watch::Receiver<(Address, Vec<Address>)> {
        &mut self.endpoints
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

const NETWORK_INTERFACES_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Returns a future that will update endpoints associated to a local address into the sender.
///
/// A local address can be bound to an "ANY" IP address, meaning that it is bound to all network
/// interfaces of the host system. This means that when the set of interfaces changes, so do local
/// endpoints. This future checks if the address is an "ANY" IP address and then continuously tracks
/// changes to the network interfaces to update the list of endpoints.
///
/// If the local address is not an "ANY" IP address, then the endpoints are updated immediately with
/// the local address and only that address and the future terminates.
///
/// In the endpoints tuple value, only the list of endpoints (the second element) is updated. The
/// first value (the local address) is never set by this function.
async fn update_address_endpoints(
    local_address: Address,
    endpoints_sender: &mut watch::Sender<(Address, Vec<Address>)>,
) {
    match local_address {
        // An "ANY" address, aka "unspecified".
        Address::Tcp {
            address: local_socket_address,
            ssl,
        } if local_socket_address.ip().is_unspecified() => {
            // Watch network interfaces changes to list all IP addresses of the host.
            let mut networks = sysinfo::Networks::new();
            loop {
                networks.refresh(true);
                let new_endpoints: Vec<_> = networks
                    .values()
                    .flat_map(|net| net.ip_networks())
                    .map(|ip_net| Address::Tcp {
                        address: SocketAddr::new(ip_net.addr, local_socket_address.port()),
                        ssl,
                    })
                    .collect();
                endpoints_sender.send_if_modified(move |(_, endpoints)| {
                    if endpoints != &new_endpoints {
                        *endpoints = new_endpoints;
                        true
                    } else {
                        false
                    }
                });
                time::sleep(NETWORK_INTERFACES_REFRESH_INTERVAL).await;
            }
        }
        // Not an any address, update endpoints and terminate.
        _ => endpoints_sender.send_modify(|(_, endpoints)| *endpoints = vec![local_address]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::Message;
    use assert_matches::assert_matches;
    use futures::{channel::mpsc, SinkExt, StreamExt};
    use qi_messaging::Body;
    use serde_json as json;
    use std::{
        collections::VecDeque,
        convert::Infallible,
        future::{ready, Future},
    };
    use tokio::spawn;

    #[derive(Clone, Copy)]
    struct DummyHandler;

    impl messaging::Handler<JsonBody> for DummyHandler {
        type Error = HandlerError;

        async fn call(
            &self,
            _address: message::Address,
            value: JsonBody,
        ) -> Result<JsonBody, Self::Error> {
            Ok(value)
        }

        fn fire_and_forget(
            &self,
            _address: message::Address,
            _request: message::FireAndForget<JsonBody>,
        ) -> impl Future<Output = ()> + Send {
            ready(())
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct JsonBody(json::Value);

    impl messaging::Body for JsonBody {
        type Error = json::Error;
        type Data = VecDeque<u8>;

        fn from_bytes(bytes: bytes::Bytes) -> Result<Self, Self::Error> {
            json::from_slice(&bytes).map(Self)
        }

        fn into_data(self) -> Result<Self::Data, Self::Error> {
            json::to_vec(&self.0).map(Into::into)
        }

        fn serialize<T>(value: &T) -> Result<Self, Self::Error>
        where
            T: serde::Serialize,
        {
            json::to_value(value).map(Self)
        }

        fn deserialize_seed<'de, T>(&'de self, seed: T) -> Result<T::Value, Self::Error>
        where
            T: serde::de::DeserializeSeed<'de>,
        {
            seed.deserialize(self.0.clone())
        }
    }

    /// The server session receives an authentication request with incompatible capabilities.
    ///
    /// It is expected that:
    ///   1. the server replies to the request with an error.
    ///   2. the connection is closed.
    #[tokio::test]
    async fn server_sends_back_error_on_client_bad_capabilities() {
        // 0.1: start the server session
        let (mut send_to_server, server_recv) = mpsc::unbounded();
        let (server_send, mut recv_from_server) = mpsc::unbounded();
        let task = spawn(Session::serve_client(
            server_recv.map(Ok::<_, Infallible>),
            server_send.sink_map_err(qi_messaging::Error::link_lost),
            auth::PermissiveAuthenticator,
            DummyHandler,
        ));

        // 0.2: start the request
        send_to_server
            .send(Message::Call {
                id: message::Id(0),
                address: control::AUTHENTICATE_ADDRESS,
                value: JsonBody::serialize(&{
                    let mut map = KeyDynValueMap::new();
                    map.set("RemoteCancelableCalls", true);
                    map.set("ObjectPtrUID", true);
                    map.set("RelativeEndpointURI", false); // A required capabilities is set to false.
                    map
                })
                .unwrap(),
            })
            .await
            .unwrap();

        // 1.
        let response = recv_from_server.next().await.unwrap();
        assert_matches!(
            response,
            Message::Error {
                address: control::AUTHENTICATE_ADDRESS,
                error,
                ..
            } => {
                assert!(error.contains("unexpected capability value"), "error is not an unexpected capability value: {error}")
            }
        );

        // 2.
        let () = task.await.unwrap();
    }

    /// The client session receives an authentication response with incompatible capabilities.
    ///
    /// It is expected that:
    ///   1. the connection is closed.
    ///   2. the error is reported back to the client user.
    #[tokio::test]
    async fn client_receives_bad_capabilities() {
        // 0.1: start the client session
        let (mut send_to_client, client_recv) = mpsc::unbounded();
        let (client_send, mut recv_from_client) = mpsc::unbounded();
        let task = spawn(Session::connect(
            client_recv.map(Ok::<_, Infallible>),
            client_send.sink_map_err(qi_messaging::Error::link_lost),
            Default::default(),
            DummyHandler,
        ));

        // 0.2: receive the request
        let request = recv_from_client.next().await.unwrap();
        assert_matches!(
            request,
            Message::Call {
                id: message::Id(1),
                address: control::AUTHENTICATE_ADDRESS,
                ..
            }
        );

        // 1: send the reply containing the capabilities
        send_to_client
            .send(Message::Reply {
                id: message::Id(1),
                address: control::AUTHENTICATE_ADDRESS,
                value: JsonBody::serialize(&{
                    let mut map = KeyDynValueMap::new();
                    map.set("RemoteCancelableCalls", true);
                    map.set("ObjectPtrUID", true);
                    map.set("RelativeEndpointURI", false); // A required capabilities is set to false.
                    map
                })
                .unwrap(),
            })
            .await
            .unwrap();

        // 1. task terminates succesfully with a result in error.
        assert!(task.await.unwrap().is_err());
    }

    /// The server expects authentication parameters, the client sends correct ones.
    ///
    /// It is expected that:
    ///   1. the authentication succeeds.
    #[tokio::test]
    async fn client_sends_good_auth_parameters() {
        let auth = auth::UserTokenAuthenticator::new("myuser".to_owned(), "mytoken".to_owned());

        // 0.1: start the server session
        let (mut send_to_server, server_recv) = mpsc::unbounded();
        let (server_send, mut recv_from_server) = mpsc::unbounded();
        spawn(Session::serve_client(
            server_recv.map(Ok::<_, Infallible>),
            server_send.sink_map_err(qi_messaging::Error::link_lost),
            auth,
            DummyHandler,
        ));

        // 0.2: start the request
        send_to_server
            .send(Message::Call {
                id: message::Id(0),
                address: control::AUTHENTICATE_ADDRESS,
                value: JsonBody::serialize(&{
                    let mut map = KeyDynValueMap::new();
                    map.set("RemoteCancelableCalls", true);
                    map.set("ObjectPtrUID", true);
                    map.set("RelativeEndpointURI", true);
                    map.set(auth::USER_KEY, "myuser");
                    map.set(auth::TOKEN_KEY, "mytoken");
                    map
                })
                .unwrap(),
            })
            .await
            .unwrap();

        // 1.
        let response = recv_from_server.next().await.unwrap();
        let body = assert_matches!(
            response,
            Message::Reply {
                address: control::AUTHENTICATE_ADDRESS,
                id: message::Id(0),
                value: body
            } => body
        );

        let mut map: KeyDynValueMap = body.deserialize().unwrap();
        let state: u32 = map
            .remove(auth::STATE_KEY)
            .unwrap_or_else(|| panic!("missing state key in map {map:?}"))
            .cast_into()
            .expect("state value is not a u32");
        assert_eq!(state, auth::STATE_DONE);
    }

    /// The client sends bad authentication parameters.
    ///
    /// It is expected that:
    ///   1. the server replies with an error.
    ///   3. the error is reported back to the user of the client.
    ///   2. the connection is closed.
    #[tokio::test]
    async fn client_send_bad_auth_parameters() {
        let auth = auth::UserTokenAuthenticator::new("myuser".to_owned(), "mytoken".to_owned());

        // 0.1: start the server session
        let (mut send_to_server, server_recv) = mpsc::unbounded();
        let (server_send, mut recv_from_server) = mpsc::unbounded();
        let task = spawn(Session::serve_client(
            server_recv.map(Ok::<_, Infallible>),
            server_send.sink_map_err(qi_messaging::Error::link_lost),
            auth,
            DummyHandler,
        ));

        // 0.2: start the request
        send_to_server
            .send(Message::Call {
                id: message::Id(0),
                address: control::AUTHENTICATE_ADDRESS,
                value: JsonBody::serialize(&{
                    let mut map = KeyDynValueMap::new();
                    map.set("RemoteCancelableCalls", true);
                    map.set("ObjectPtrUID", true);
                    map.set("RelativeEndpointURI", true);
                    map.set(auth::USER_KEY, "myuser");
                    map.set(auth::TOKEN_KEY, "badtoken"); // token is not correct
                    map
                })
                .unwrap(),
            })
            .await
            .unwrap();

        // 1.
        let response = recv_from_server.next().await.unwrap();
        let error = assert_matches!(
            response,
            Message::Error {
                address: control::AUTHENTICATE_ADDRESS,
                error,
                ..
            } => error
        );
        assert!(
            error.contains("failure to verify authentication request"),
            "error is not an authentication failure: {error}"
        );

        // 2.
        let () = task.await.unwrap();
    }
}
