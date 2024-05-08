use super::{
    authentication::{self, Authenticator},
    capabilities,
};
use crate::{
    messaging::{self, CapabilitiesMap},
    Error,
};
use async_trait::async_trait;
use bytes::Bytes;
use messaging::message;
use qi_value::{ActionId, Dynamic, ObjectId, ServiceId, Value};
use sealed::sealed;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub(super) const SERVICE_ID: ServiceId = ServiceId(0);
pub(super) const OBJECT_ID: ObjectId = ObjectId(0);
pub(super) const AUTHENTICATE_ACTION_ID: ActionId = ActionId(8);

pub(super) fn is_addressed_by(address: message::Address) -> bool {
    address.service() == SERVICE_ID && address.object() == OBJECT_ID
}

const AUTHENTICATE_ADDRESS: message::Address =
    message::Address::new(SERVICE_ID, OBJECT_ID, AUTHENTICATE_ACTION_ID);

#[derive(Debug)]
pub(super) struct Control<A> {
    authenticator: A,
    capabilities: Arc<Mutex<Option<CapabilitiesMap>>>,
    remote_authorized: bool,
}

impl<A> Control<A> {
    pub(super) fn new(
        authenticator: A,
        capabilities: Arc<Mutex<Option<CapabilitiesMap>>>,
        remote_authorized: bool,
    ) -> Self {
        Self {
            authenticator,
            capabilities,
            remote_authorized,
        }
    }
}

impl<A> Control<A>
where
    A: Authenticator,
{
    pub(super) async fn authenticate(
        &mut self,
        request: CapabilitiesMap,
    ) -> Result<CapabilitiesMap, Error> {
        let shared_capabilities = capabilities::local_map().clone().intersected_with(&request);
        let parameters = request.into_iter().map(|(k, v)| (k, v.0)).collect();
        self.authenticator.verify(parameters)?;
        capabilities::check_required(&shared_capabilities)?;
        self.capabilities
            .lock()
            .await
            .replace(shared_capabilities.clone());
        self.remote_authorized = true;
        Ok(authentication::state_done_map(shared_capabilities))
    }

    pub(super) fn remote_authorized(&self) -> bool {
        self.remote_authorized
    }
}

#[sealed]
#[async_trait]
pub(super) trait AuthenticateService {
    async fn authenticate(
        &mut self,
        parameters: HashMap<String, Value<'_>>,
    ) -> Result<CapabilitiesMap, Error>;
}

#[sealed]
#[async_trait]
impl AuthenticateService for messaging::Client<Bytes, Bytes> {
    async fn authenticate(
        &mut self,
        parameters: HashMap<String, Value<'_>>,
    ) -> Result<CapabilitiesMap, Error> {
        let mut request = capabilities::local_map().clone();
        request.extend(
            parameters
                .into_iter()
                .map(|(k, v)| (k, Dynamic(v.into_owned()))),
        );
        let arg = request.serialize_value()?;
        let result = self
            .ready()
            .await?
            .call((AUTHENTICATE_ADDRESS, arg))
            .await?
            .deserialize_value()?;
        Ok(result)
    }
}
