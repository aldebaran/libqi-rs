pub mod authentication;
pub(crate) mod capabilities;
mod control;

use self::{authentication::Authenticator, control::ControlService};
use crate::{channel, error::ConnectionError, Address, Error};
use bytes::Bytes;
use format::{de::BufExt, ser::IntoValueExt};
use futures::{future::BoxFuture, stream::FusedStream, Future, FutureExt, SinkExt, StreamExt};
use messaging::{message, CapabilitiesMap};
use qi_format as format;
use qi_messaging as messaging;
use qi_value::{FromValue, IntoValue, Reflect, Value};
use std::{collections::HashMap, future::ready, sync::Arc};
use tokio::{select, sync::RwLock};
use tracing::{debug_span, instrument, Instrument};

pub(crate) async fn connect<'service, 'value, Svc>(
    address: Address,
    service: Svc,
    authentication_parameters: Option<HashMap<String, Value<'value>>>,
) -> Result<
    (
        impl Future<Output = Result<Client, Error>> + Send + 'value,
        impl Future<Output = Result<(), ConnectionError>> + Send + 'service,
    ),
    Error,
>
where
    Svc: messaging::Service + Send + 'service,
{
    let (messages_read, mut messages_write) = channel::open(address.clone()).await?;
    let (mut messages_in_sender, messages_in_receiver) = futures::channel::mpsc::channel(1);
    let capabilities = Arc::new(RwLock::new(None));
    let service = Service::new(
        authentication::PermissiveAuthenticator,
        Arc::clone(&capabilities),
        service,
    );
    let (mut endpoint, client) = messaging::endpoint(messages_in_receiver, service);

    let session = async move {
        let control_client = control::Client::new(client.clone());
        let resolved_capabilities = control_client
            .authenticate(authentication_parameters.unwrap_or_default())
            .instrument(debug_span!("authenticate"))
            .await?;
        capabilities::check_required(&resolved_capabilities)?;
        *capabilities.write().await = Some(resolved_capabilities);
        Ok(Client {
            address,
            client,
            capabilities,
        })
    };

    let mut messages_read = messages_read.fuse();
    let connection = async move {
        loop {
            select! {
                Some(msg) = messages_read.next(), if !messages_read.is_terminated() => {
                    let _res = messages_in_sender.send(msg?).await;
                    debug_assert!(_res.is_ok());
                }
                Some(msg) = endpoint.next_message() => {
                    messages_write.send(msg).await?;
                }
            }
        }
    };

    Ok((session, connection))
}

#[derive(Clone, Debug)]
pub struct Client {
    address: Address,
    client: messaging::Client,
    capabilities: Arc<RwLock<Option<CapabilitiesMap>>>,
}

impl Client {
    pub(crate) fn call_into_value<'t, T, R>(
        &self,
        address: message::Address,
        args: T,
    ) -> impl Future<Output = Result<R, Error>>
    where
        T: IntoValue<'t> + std::fmt::Debug,
        R: Reflect + FromValue<'static>,
    {
        let call = self.call(address, args);
        async { Ok(call.await?.deserialize_value()?) }
    }

    pub(crate) fn call<'t, T>(
        &self,
        address: message::Address,
        args: T,
    ) -> impl Future<Output = Result<Bytes, Error>>
    where
        T: IntoValue<'t>,
    {
        use messaging::Service;
        let call = args
            .serialize_value()
            .map(|value| self.client.call(messaging::Call::new(address, value)));
        async move { Ok(call?.await?) }
            .instrument(debug_span!("call", url = %self.address, %address))
    }

    pub(crate) fn post<'t, T>(
        &self,
        address: message::Address,
        args: T,
    ) -> impl Future<Output = Result<(), Error>>
    where
        T: IntoValue<'t>,
    {
        use messaging::Service;
        let post = args
            .serialize_value()
            .map(|value| self.client.post(messaging::Post::new(address, value)));
        async move { Ok(post?.await?) }
            .instrument(debug_span!("post", url = %self.address, %address))
    }

    #[instrument(level = "debug", skip(self, args))]
    pub(crate) fn event<'t, T>(
        &self,
        address: message::Address,
        args: T,
    ) -> impl Future<Output = Result<(), Error>>
    where
        T: IntoValue<'t>,
    {
        use messaging::Service;
        let event = args
            .serialize_value()
            .map(|value| self.client.event(messaging::Event::new(address, value)));
        async move { Ok(event?.await?) }
            .instrument(debug_span!("event", url = %self.address, %address))
    }
}

#[derive(Debug)]
struct Service<S> {
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
