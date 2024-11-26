pub mod authentication;
mod capabilities;
pub mod control;
mod map;
mod target;

pub(crate) use self::map::Map;
pub use self::target::Target;
use crate::{
    error::{Error, FormatError, HandlerError},
    messaging::{self, message},
    session::authentication::{Authenticator, PermissiveAuthenticator},
    value::{self, KeyDynValueMap},
};
use control::Control;
use futures::{stream::FusedStream, Sink, StreamExt, TryStream};
use qi_messaging::Address;
use std::{net::SocketAddr, pin::pin};
use tokio::{select, sync::watch, task, time};

pub struct Session<Body> {
    capabilities: watch::Receiver<Option<KeyDynValueMap>>,
    client: messaging::Client<Body>,
}

impl<Body> Session<Body>
where
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    pub(crate) async fn connect<Handler>(
        address: messaging::Address,
        credentials: KeyDynValueMap,
        handler: Handler,
    ) -> Result<Self, Error>
    where
        Handler: messaging::Handler<Body, Error = HandlerError> + Send + Sync + 'static,
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

    pub async fn serve_client<Auth, MsgStream, MsgSink, Handler>(
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
/// In the tuple value, only the list of endpoints is updated. The first value (he local address) is
/// never set by this function. It is the responsibility of the caller to set it.
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
                networks.refresh_list();
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
