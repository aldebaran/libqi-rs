pub mod authentication;
pub(crate) mod capabilities;
mod control;

use self::{authentication::Authenticator, control::ControlService};
use crate::{Address, Error};
use bytes::Bytes;
use format::{de::BufExt, ser::IntoValueExt};
use futures::{future::BoxFuture, Future, FutureExt};
use messaging::{message, CapabilitiesMap};
use qi_format as format;
use qi_messaging as messaging;
use qi_value::{Type, Value};
use std::{
    future::ready,
    sync::{Arc, Weak},
};
use tokio::sync::RwLock;
use tracing::{debug_span, Instrument};

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub addresses: Vec<Address>,
    pub credentials: authentication::Parameters,
}

impl Config {
    pub fn add_addresses<A>(mut self, address: A) -> Self
    where
        A: IntoIterator,
        A::Item: Into<Address>,
    {
        self.addresses.extend(address.into_iter().map(Into::into));
        self
    }

    pub fn add_credentials_parameter(mut self, key: String, value: Value<'static>) -> Self {
        self.credentials.insert(key, value);
        self
    }
}

pub(crate) async fn connect(
    client: messaging::Client,
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
        client,
        capabilities,
    })
}

#[derive(Clone, Debug)]
pub struct Client {
    client: messaging::Client,
    capabilities: Arc<RwLock<Option<CapabilitiesMap>>>,
}

impl Client {
    pub(crate) fn call(
        &self,
        address: message::Address,
        args: Value<'_>,
        return_type: Option<Type>,
    ) -> impl Future<Output = Result<Value<'static>, Error>> {
        use messaging::Service;
        let call = format::to_bytes(&args)
            .map(|value| self.client.call(messaging::Call::new(address, value)));
        async move {
            Ok(call?
                .await?
                .deserialize_value_of_type(return_type.as_ref())?)
        }
    }

    pub(crate) fn post(
        &self,
        address: message::Address,
        args: Value<'_>,
    ) -> impl Future<Output = Result<(), Error>> {
        use messaging::Service;
        let post = format::to_bytes(&args)
            .map(|value| self.client.post(messaging::Post::new(address, value)));
        async move { Ok(post?.await?) }
    }

    pub(crate) fn event(
        &self,
        address: message::Address,
        args: Value<'_>,
    ) -> impl Future<Output = Result<(), Error>> {
        use messaging::Service;
        let event = format::to_bytes(&args)
            .map(|value| self.client.event(messaging::Event::new(address, value)));
        async move { Ok(event?.await?) }
    }

    pub(crate) fn downgrade(&self) -> WeakClient {
        WeakClient {
            client: self.client.downgrade(),
            capabilities: Arc::downgrade(&self.capabilities),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WeakClient {
    client: messaging::WeakClient,
    capabilities: Weak<RwLock<Option<CapabilitiesMap>>>,
}

impl WeakClient {
    pub(crate) fn upgrade(&self) -> Option<Client> {
        Some(Client {
            client: self.client.upgrade()?,
            capabilities: self.capabilities.upgrade()?,
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

    fn unlock_inner(&mut self) {
        self.inner.unlock()
    }
}

impl<S> messaging::Service for Service<S>
where
    S: messaging::Service + Send,
{
    fn call(
        &self,
        mut call: messaging::Call,
    ) -> BoxFuture<'static, Result<Bytes, messaging::Error>> {
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
            ready(Err(Error::NoHandler(address).into())).boxed()
        }
    }

    fn post(&self, post: messaging::Post) -> BoxFuture<'static, Result<(), messaging::Error>> {
        if let Some(svc) = self.inner.get() {
            svc.post(post)
        } else {
            ready(Err(Error::NoHandler(post.address()).into())).boxed()
        }
    }

    fn event(&self, event: messaging::Event) -> BoxFuture<'static, Result<(), messaging::Error>> {
        if let Some(svc) = self.inner.get() {
            svc.event(event)
        } else {
            ready(Err(Error::NoHandler(event.address()).into())).boxed()
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

    fn unlock(&mut self) {
        self.unlocked = true;
    }
}
