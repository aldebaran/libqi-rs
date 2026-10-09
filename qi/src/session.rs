pub mod auth;
mod capabilities;
pub(crate) mod control;
mod map;
mod target;

use self::auth::PermissiveAuthenticator;
pub(crate) use self::{auth::Authenticator, map::Map, target::Target};
use crate::{
    error::{Error, FormatError},
    format,
    messaging::{self, message},
    value::{self, KeyDynValueMap},
};
use control::{Control, SessionHandler};
use futures::{future::FusedFuture, stream::FusedStream, FutureExt, Sink, StreamExt, TryStream};
use qi_messaging::Address;
use std::{net::SocketAddr, pin::pin};
use tokio::{select, sync::watch, task, time};

pub(crate) struct Session {
    capabilities: watch::Receiver<Option<KeyDynValueMap>>,
    client: messaging::Client,
}

impl Session {
    pub(crate) async fn connect<MsgStream, MsgSink, Handler>(
        messages_stream: MsgStream,
        messages_sink: MsgSink,
        credentials: KeyDynValueMap,
        handler: Handler,
    ) -> Result<Self, Error>
    where
        MsgStream: TryStream<Ok = messaging::Message> + Send + 'static,
        MsgStream::Error: Send,
        MsgSink: Sink<messaging::Message> + Send + 'static,
        Handler: SessionHandler + Clone,
    {
        let Control {
            controller,
            capabilities,
            handler,
            ..
        } = control::make(handler, PermissiveAuthenticator, true);
        let (client, connection) =
            messaging::endpoint::start(messages_stream, messages_sink, handler);
        task::spawn(async move {
            let _res = connection.await;
        });
        controller
            .authenticate_to_server(&client, credentials)
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
        Handler: SessionHandler + Clone,
    {
        let (clients, local_address) = messaging::channel::serve(address).await?;
        let (mut endpoints_sender, endpoints_receiver) =
            watch::channel((local_address, Vec::new()));
        let task = task::spawn(async move {
            let mut clients = pin!(clients.fuse());
            let mut update_endpoints =
                pin!(update_address_endpoints(local_address, &mut endpoints_sender).fuse());
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
                    () = &mut update_endpoints, if !update_endpoints.is_terminated() => {
                        // A concrete address finishes this future immediately. Leave it out of
                        // later polls so the server task keeps accepting clients.
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
        MsgStream: TryStream<Ok = messaging::Message> + Send + 'static,
        MsgStream::Error: Send,
        MsgSink: Sink<messaging::Message> + Send + 'static,
        Auth: Authenticator + Send + Sync + 'static,
        Handler: SessionHandler + Clone,
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
        let args = format::to_bytes(&value).map_err(FormatError::ArgumentsSerialization)?;
        let reply = self.client.call(address, args).await?;
        Ok(value::deserialize(return_type, &reply)
            .map_err(FormatError::MethodReturnValueDeserialization)?
            .into_owned())
    }

    pub(crate) async fn post(
        &self,
        address: message::Address,
        args: value::Value<'_>,
    ) -> Result<(), Error> {
        let args = format::to_bytes(&args).map_err(FormatError::ArgumentsSerialization)?;
        self.client.post(address, args).await?;
        Ok(())
    }

    pub(crate) async fn send_event(
        &self,
        address: message::Address,
        value: value::Value<'_>,
    ) -> Result<(), Error> {
        let value = format::to_bytes(&value).map_err(FormatError::ArgumentsSerialization)?;
        self.client.send_event(address, value).await?;
        Ok(())
    }

    pub(crate) fn downgrade(&self) -> WeakSession {
        WeakSession {
            capabilities: self.capabilities.clone(),
            client: self.client.downgrade(),
        }
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            capabilities: self.capabilities.clone(),
            client: self.client.clone(),
        }
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("capabilities", &self.capabilities)
            .field("client", &self.client)
            .finish()
    }
}

pub(crate) struct WeakSession {
    capabilities: watch::Receiver<Option<KeyDynValueMap>>,
    client: messaging::WeakClient,
}

impl WeakSession {
    pub(crate) fn upgrade(&self) -> Option<Session> {
        self.client.upgrade().map(|client| Session {
            capabilities: self.capabilities.clone(),
            client,
        })
    }
}

impl Clone for WeakSession {
    fn clone(&self) -> Self {
        Self {
            capabilities: self.capabilities.clone(),
            client: self.client.clone(),
        }
    }
}

impl std::fmt::Debug for WeakSession {
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
    use crate::{messaging::Message, HandlerError};
    use assert_matches::assert_matches;
    use bytes::Bytes;
    use futures::{
        channel::mpsc,
        future::{ok, Ready},
        SinkExt, StreamExt,
    };
    use qi_messaging::message::KeyDynValueMap;
    use std::convert::Infallible;
    use tokio::spawn;

    #[derive(Clone, Copy)]
    struct DummyHandler;

    impl messaging::CallHandler for DummyHandler {
        type Error = HandlerError;
        type Future = Ready<Result<Bytes, Self::Error>>;

        fn handle_call(&self, _address: message::Address, args: Bytes) -> Self::Future {
            ok(args)
        }
    }

    impl messaging::EventHandler for DummyHandler {
        fn handle_event(&self, _address: message::Address, _args: Bytes) {}
    }

    impl messaging::PostHandler for DummyHandler {
        fn handle_post(&self, _address: message::Address, _args: Bytes) {}
    }

    impl messaging::CapabilitiesHandler for DummyHandler {
        fn handle_capabilities(&self, _address: message::Address, _capabilities: KeyDynValueMap) {}
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
                payload: format::to_bytes(&{
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
                payload: format::to_bytes(&{
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
                payload: format::to_bytes(&{
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
        let payload = assert_matches!(
            response,
            Message::Reply {
                address: control::AUTHENTICATE_ADDRESS,
                id: message::Id(0),
                payload
            } => payload
        );

        let mut map: KeyDynValueMap = format::from_slice(&payload).unwrap();
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
                payload: format::to_bytes(&{
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
