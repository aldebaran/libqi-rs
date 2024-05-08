mod address;
pub(crate) mod authentication;
mod cache;
mod capabilities;
mod channel;
mod control;
mod message_format;
mod reference;

pub(crate) use self::cache::Cache;
pub use self::{address::Address, reference::Reference};
use self::{channel::Connection, control::AuthenticateService};
use crate::{
    error::NoMessageHandlerError,
    messaging::{self, message, CapabilitiesMap},
    value::{Type, Value},
    Authenticator, Error, PermissiveAuthenticator,
};
use bytes::Bytes;
use futures::{stream::FuturesUnordered, Future, FutureExt, StreamExt};
use std::{future::ready, pin::pin, sync::Arc};
use tokio::{select, sync::Mutex};

#[derive(Clone, Debug)]
pub(crate) struct Session {
    uid: Uid,
    capabilities: Arc<Mutex<Option<CapabilitiesMap>>>,
    client: messaging::Client<Bytes, Bytes>,
}

impl Session {
    pub(crate) async fn connect<'a, Svc>(
        address: Address,
        credentials: authentication::Parameters,
        service: Svc,
    ) -> Result<(Self, Connection<'a>), Error>
    where
        Svc: tower::Service<(message::Address, Bytes)> + 'a,
    {
        let capabilities = Arc::default();
        let service = Service::client(service, Arc::clone(&capabilities));
        let (client, connection) = channel::connect(address, service).await?;
        let shared_capabilities = client.authenticate(credentials).await?;
        capabilities::check_required(&shared_capabilities)?;
        Ok((
            Session {
                uid: Uid::new(),
                capabilities,
                client,
            },
            connection,
        ))
    }

    pub(crate) fn call(
        &self,
        address: message::Address,
        args: Value<'_>,
        return_type: Option<Type>,
    ) -> impl Future<Output = Result<Value<'static>, Error>> {
        let call = qi_format::to_bytes(&args)
            .map(|value| self.client.call(messaging::Call::new(address, value)));
        async move {
            Ok(call?
                .await?
                .deserialize_value_of_type(return_type.as_ref())?)
        }
    }

    // pub(crate) fn post(
    //     &self,
    //     address: message::Address,
    //     args: Value<'_>,
    // ) -> impl Future<Output = Result<(), Error>> {
    //     use messaging::Service;
    //     let post = qi_format::to_bytes(&args)
    //         .map(|value| self.client.post(messaging::Post::new(address, value)));
    //     async move { Ok(post?.await?) }
    // }

    // pub(crate) fn event(
    //     &self,
    //     address: message::Address,
    //     args: Value<'_>,
    // ) -> impl Future<Output = Result<(), Error>> {
    //     use messaging::Service;
    //     let event = qi_format::to_bytes(&args)
    //         .map(|value| self.client.event(messaging::Event::new(address, value)));
    //     async move { Ok(event?.await?) }
    // }

    pub fn uid(&self) -> Uid {
        self.uid.clone()
    }
}

pub(crate) async fn serve<Svc, A>(
    address: Address,
    service: Svc,
    authenticator: A,
) -> Result<(impl Future<Output = ()> + Send, Vec<Address>), std::io::Error>
where
    Svc: tower::Service<(message::Address, Bytes)> + Clone,
    A: Authenticator,
{
    let make_service = move || {
        let capabilities = Arc::default();
        Service::server(service, authenticator, capabilities)
    };
    let (clients, endpoints) = channel::serve(address, make_service).await?;
    let server = async move {
        let mut connections = FuturesUnordered::new();
        pin!(clients);
        loop {
            select! {
                Some((client, connection)) = clients.next(), if !clients.is_terminated() => {
                    connections.push(async move {
                        connection.await?;
                        drop(client);
                    });
                }
                _res = connections.next(), if !connections.is_terminated() => {
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

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "crate::value", transparent)]
// TODO: value(`as = "DisplayFromStr"`)
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_string(s.to_owned()))
    }
}

#[derive(Debug)]
struct Service<S, A> {
    inner: S,
    control: control::Control<A>,
}

impl<S, A> Service<S, A> {
    fn server(
        inner: S,
        authenticator: A,
        capabilities: Arc<Mutex<Option<CapabilitiesMap>>>,
    ) -> Self {
        Self::new(inner, authenticator, capabilities, false)
    }

    fn new(
        inner: S,
        authenticator: A,
        capabilities: Arc<Mutex<Option<CapabilitiesMap>>>,
        remote_authorized: bool,
    ) -> Self {
        Self {
            inner,
            control: control::Control::new(authenticator, capabilities, remote_authorized),
        }
    }

    fn inner_if_unlocked(&self) -> Option<&S> {
        self.control.remote_authorized().then_some(&self.inner)
    }
}

impl<S> Service<S, PermissiveAuthenticator> {
    fn client(inner: S, capabilities: Arc<Mutex<Option<CapabilitiesMap>>>) -> Self {
        Self::new(inner, PermissiveAuthenticator, capabilities, true)
    }
}

impl<S, A> tower::Service<(message::Address, Bytes)> for Service<S, A>
where
    S: tower::Service<(message::Address, Bytes)>,
    A: Authenticator,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ServiceCallFuture;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&self, (address, value): (message::Address, Bytes)) -> Self::Future {
        let address = call.address();
        if control::is_addressed_by(address) {
            let authenticate = call
                .value_mut()
                .deserialize_value()
                .map(|request| self.control.authenticate(request));
            async move { Ok(authenticate?.await?.serialize_value()?) }.boxed()
        } else if let Some(inner) = self.inner_if_unlocked() {
            inner.call(call)
        } else {
            ready(Err(NoMessageHandlerError(address).into())).boxed()
        }
    }
}

struct ServiceCallFuture;

// fn post(&self, post: messaging::Post) -> BoxFuture<'static, Result<(), messaging::Error>> {
//     if let Some(inner) = self.inner_if_unlocked() {
//         inner.post(post)
//     } else {
//         ready(Err(NoMessageHandlerError(post.address()).into())).boxed()
//     }
// }

// fn event(&self, event: messaging::Event) -> BoxFuture<'static, Result<(), messaging::Error>> {
//     if let Some(inner) = self.inner_if_unlocked() {
//         inner.event(event)
//     } else {
//         ready(Err(NoMessageHandlerError(event.address()).into())).boxed()
//     }
// }
