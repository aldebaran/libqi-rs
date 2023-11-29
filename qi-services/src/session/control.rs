use super::{
    authentication::{Authenticator, CapabilitiesMapExt},
    capabilities,
};
use crate::error::Error;
use futures::{future::BoxFuture, FutureExt};
use messaging::{message, Service};
use qi_format::{de::BufExt, ser::IntoValueExt};
use qi_messaging::{
    self as messaging, CapabilitiesMap, CapabilitiesMapExt as MessagingCapabilitiesMapExt,
};
use qi_value::{ActionId, Dynamic, ObjectId, ServiceId, Value};
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::sync::RwLock;

pub(super) const SERVICE_ID: ServiceId = ServiceId(0);
pub(super) const OBJECT_ID: ObjectId = ObjectId(0);
pub(super) const AUTHENTICATE_ACTION_ID: ActionId = ActionId(8);

pub(super) fn is_addressed_by(address: message::Address) -> bool {
    address.service() == SERVICE_ID && address.object() == OBJECT_ID
}

const fn authenticate_address() -> message::Address {
    message::Address::new(SERVICE_ID, OBJECT_ID, AUTHENTICATE_ACTION_ID)
}

pub(crate) trait ControlService {
    type Future: std::future::Future<Output = Result<CapabilitiesMap, Error>>;

    fn call_authenticate(&self, request: CapabilitiesMap) -> Self::Future;
}

pub(super) struct Control {
    authenticator: Box<dyn Authenticator + Send + Sync + 'static>,
    capabilities: Arc<RwLock<Option<CapabilitiesMap>>>,
}

impl Control {
    pub(super) fn new<A>(
        authenticator: A,
        capabilities: Arc<RwLock<Option<CapabilitiesMap>>>,
    ) -> Self
    where
        A: Authenticator + Send + Sync + 'static,
    {
        Self {
            authenticator: Box::new(authenticator),
            capabilities,
        }
    }

    fn authenticate(
        &self,
        request: CapabilitiesMap,
    ) -> impl Future<Output = Result<CapabilitiesMap, Error>> {
        let mut resolved_capabilities =
            capabilities::local_map().clone().intersected_with(&request);
        let parameters = request.into_iter().map(|(k, v)| (k, v.0)).collect();
        let result = self.authenticator.verify(parameters);
        let capabilities = Arc::clone(&self.capabilities);
        async move {
            result?;
            *capabilities.write().await = Some(resolved_capabilities.clone());
            resolved_capabilities.insert_authentication_state_done();
            Ok(resolved_capabilities)
        }
    }
}

impl std::fmt::Debug for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server({:?})", self.capabilities)
    }
}

impl ControlService for Control {
    type Future = BoxFuture<'static, Result<CapabilitiesMap, Error>>;

    fn call_authenticate(&self, request: CapabilitiesMap) -> Self::Future {
        self.authenticate(request).boxed()
    }
}

#[derive(Debug)]
pub(crate) struct Client(messaging::Client);

impl Client {
    pub(crate) fn new(client: messaging::Client) -> Self {
        Self(client)
    }

    pub(crate) fn authenticate(
        &self,
        parameters: HashMap<String, Value<'_>>,
    ) -> impl Future<Output = Result<CapabilitiesMap, Error>> {
        let mut request = capabilities::local_map().clone();
        request.extend(
            parameters
                .into_iter()
                .map(|(k, v)| (k, Dynamic(v.into_owned()))),
        );
        self.call_authenticate(request)
    }
}

impl ControlService for Client {
    type Future = BoxFuture<'static, Result<CapabilitiesMap, Error>>;

    fn call_authenticate(&self, request: CapabilitiesMap) -> Self::Future {
        let call = request.serialize_value().map(|value| {
            self.0
                .call(messaging::Call::new(authenticate_address(), value))
        });
        async move { Ok(call?.await?.deserialize_value()?) }.boxed()
    }
}
