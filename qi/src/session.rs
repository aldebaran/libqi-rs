pub(crate) mod authentication;
mod capabilities;
mod control;
mod error;
pub mod reference;
mod registry;

pub(crate) use self::{authentication::Authenticator, registry::Registry};
use crate::{error::NoMessageHandlerError, Error};
use bytes::Bytes;
use futures::{future::BoxFuture, Future, FutureExt};
use qi_format::{de::BufExt, ser::IntoValueExt};
use qi_messaging::{message, CapabilitiesMap};
use qi_value::{Type, Value};
pub use reference::Reference;
use std::{future::ready, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug_span, Instrument};

pub(crate) async fn connect(
    client: qi_messaging::Client,
    credentials: authentication::Parameters,
    capabilities: Arc<RwLock<Option<CapabilitiesMap>>>,
) -> Result<Client, Error> {
    let control_client = control::Client::new(client.clone());
    let resolved_capabilities = control_client
        .authenticate(credentials)
        .instrument(debug_span!("authenticate"))
        .await?;
    capabilities::check_required(&resolved_capabilities)?;
    *capabilities.write().await = Some(resolved_capabilities);
    Ok(Client {
        uid: Uid::new(),
        client,
        // capabilities,
    })
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "qi_value", transparent)]
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

#[derive(Clone, Debug)]
pub struct Client {
    uid: Uid,
    client: qi_messaging::Client,
    // capabilities: Arc<RwLock<Option<CapabilitiesMap>>>,
}

impl Client {
    pub(crate) fn call(
        &self,
        address: message::Address,
        args: Value<'_>,
        return_type: Option<Type>,
    ) -> impl Future<Output = Result<Value<'static>, Error>> {
        use qi_messaging::Service;
        let call = qi_format::to_bytes(&args)
            .map(|value| self.client.call(qi_messaging::Call::new(address, value)));
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
    //     use qi_messaging::Service;
    //     let post = qi_format::to_bytes(&args)
    //         .map(|value| self.client.post(qi_messaging::Post::new(address, value)));
    //     async move { Ok(post?.await?) }
    // }

    // pub(crate) fn event(
    //     &self,
    //     address: message::Address,
    //     args: Value<'_>,
    // ) -> impl Future<Output = Result<(), Error>> {
    //     use qi_messaging::Service;
    //     let event = qi_format::to_bytes(&args)
    //         .map(|value| self.client.event(qi_messaging::Event::new(address, value)));
    //     async move { Ok(event?.await?) }
    // }

    pub fn uid(&self) -> Uid {
        self.uid.clone()
    }

    // pub(crate) fn downgrade(&self) -> WeakClient {
    //     WeakClient {
    //         client: self.client.downgrade(),
    //         capabilities: Arc::downgrade(&self.capabilities),
    //     }
    // }
}

#[derive(Clone, Debug)]
pub(crate) struct WeakClient {
    id: Uid,
    client: qi_messaging::WeakClient,
    // capabilities: Weak<RwLock<Option<CapabilitiesMap>>>,
}

impl WeakClient {
    pub(crate) fn upgrade(&self) -> Option<Client> {
        Some(Client {
            uid: self.id.clone(),
            client: self.client.upgrade()?,
            // capabilities: self.capabilities.upgrade()?,
        })
    }
}

#[derive(Debug)]
pub(crate) struct Service<S> {
    control: control::Control,
    inner: InnerService<S>,
}

impl<T> Service<T> {
    pub(crate) fn new<A>(
        authenticator: A,
        capabilities: Arc<RwLock<Option<CapabilitiesMap>>>,
        underlying: T,
    ) -> Self
    where
        A: Authenticator + Send + Sync + 'static,
    {
        Self {
            control: control::Control::new(authenticator, capabilities),
            inner: InnerService::new(true, underlying),
        }
    }

    // fn unlock_inner(&mut self) {
    //     self.inner.unlock()
    // }
}

impl<T> qi_messaging::Service for Service<T>
where
    T: qi_messaging::Service,
{
    fn call(
        &self,
        mut call: qi_messaging::Call,
    ) -> BoxFuture<'static, Result<Bytes, qi_messaging::Error>> {
        use self::control::ControlService;
        let address = call.address();
        if control::is_addressed_by(address) {
            let authenticate = call
                .value_mut()
                .deserialize_value()
                .map(|request| self.control.call_authenticate(request));
            async move { Ok(authenticate?.await?.serialize_value()?) }.boxed()
        } else if let Some(svc) = self.inner.get() {
            svc.call(call)
        } else {
            ready(Err(NoMessageHandlerError(address).into())).boxed()
        }
    }

    fn post(
        &self,
        post: qi_messaging::Post,
    ) -> BoxFuture<'static, Result<(), qi_messaging::Error>> {
        if let Some(svc) = self.inner.get() {
            svc.post(post)
        } else {
            ready(Err(NoMessageHandlerError(post.address()).into())).boxed()
        }
    }

    fn event(
        &self,
        event: qi_messaging::Event,
    ) -> BoxFuture<'static, Result<(), qi_messaging::Error>> {
        if let Some(svc) = self.inner.get() {
            svc.event(event)
        } else {
            ready(Err(NoMessageHandlerError(event.address()).into())).boxed()
        }
    }
}

#[derive(Debug)]
struct InnerService<T> {
    unlocked: bool,
    service: T,
}

impl<T> InnerService<T> {
    fn new(unlocked: bool, service: T) -> Self {
        Self { unlocked, service }
    }

    fn get(&self) -> Option<&T> {
        self.unlocked.then_some(&self.service)
    }

    // fn unlock(&mut self) {
    //     self.unlocked = true;
    // }
}
