use super::{
    authentication::{self, Authenticator},
    capabilities,
};
use crate::{
    format,
    messaging::{self, CapabilitiesMap},
    value::IntoValue,
    Error,
};
use bytes::Bytes;
use messaging::message;
use qi_value::{ActionId, Dynamic, ObjectId, ServiceId, Value};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tower::{Service, ServiceExt};

pub(super) const SERVICE_ID: ServiceId = ServiceId(0);
pub(super) const OBJECT_ID: ObjectId = ObjectId(0);
pub(super) const AUTHENTICATE_ACTION_ID: ActionId = ActionId(8);

pub(super) fn is_addressed_by(address: message::Address) -> bool {
    address.service() == SERVICE_ID && address.object() == OBJECT_ID
}

const AUTHENTICATE_ADDRESS: message::Address =
    message::Address(SERVICE_ID, OBJECT_ID, AUTHENTICATE_ACTION_ID);

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
        let shared_capabilities = capabilities::shared_with_local(&request);
        capabilities::check_required(&shared_capabilities)?;
        let parameters = request.into_iter().map(|(k, v)| (k, v.0)).collect();
        self.authenticator.verify(parameters)?;
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

async fn client_send_authenticate<R>(
    client: &mut messaging::Client<Bytes, R>,
    parameters: HashMap<String, Value<'_>>,
) -> Result<CapabilitiesMap, Error>
where
    R: bytes::Buf + Send,
{
    let mut request = capabilities::local_map().clone();
    request.extend(
        parameters
            .into_iter()
            .map(|(k, v)| (k, Dynamic(v.into_owned()))),
    );
    let arg = format::to_bytes(&request.into_value())?;
    let result_buf = client
        .ready()
        .await?
        .call((AUTHENTICATE_ADDRESS, arg))
        .await?;
    Ok(format::from_buf(result_buf)?)
}
