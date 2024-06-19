use super::{
    authentication::{self, Authenticator},
    capabilities,
};
use crate::{
    messaging::{self, CapabilitiesMap},
    Error, Result,
};
use futures::{future, FutureExt, TryFutureExt};
use messaging::message;
use qi_value::{ActionId, Dynamic, ObjectId, ServiceId, Value};
use serde::Deserialize;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::sync::watch;

const SERVICE_ID: ServiceId = ServiceId(0);
const OBJECT_ID: ObjectId = ObjectId(0);
const AUTHENTICATE_ACTION_ID: ActionId = ActionId(8);

fn is_control_address(address: message::Address) -> bool {
    address.service() == SERVICE_ID && address.object() == OBJECT_ID
}

const AUTHENTICATE_ADDRESS: message::Address =
    message::Address(SERVICE_ID, OBJECT_ID, AUTHENTICATE_ACTION_ID);

pub(super) struct Controller {
    authenticator: Box<dyn Authenticator + Send + Sync>,
    capabilities: watch::Sender<Option<CapabilitiesMap<'static>>>,
    remote_authorized: watch::Sender<bool>,
}

impl Controller {
    fn authenticate(&self, request: CapabilitiesMap<'_>) -> Result<CapabilitiesMap<'static>> {
        let shared_capabilities = capabilities::shared_with_local(&request);
        capabilities::check_required(&shared_capabilities)?;
        let parameters = request.into_iter().map(|(k, v)| (k, v.0)).collect();
        self.authenticator.verify(parameters)?;
        self.capabilities
            .send_replace(Some(shared_capabilities.clone()));
        self.remote_authorized.send_replace(true);
        Ok(authentication::state_done_map(shared_capabilities))
    }

    pub(super) async fn authenticate_to_remote<T, R>(
        &self,
        client: &mut messaging::Client<T, R>,
        parameters: HashMap<String, Value<'_>>,
    ) -> Result<()>
    where
        T: messaging::BodyBuf + Send,
        T::Error: Into<Error>,
        R: messaging::BodyBuf + Send,
        R::Error: Into<Error>,
        for<'de> <R::Deserializer<'de> as serde::Deserializer<'de>>::Error: Into<R::Error>,
    {
        self.capabilities.send_replace(None); // Reset the capabilities
        let mut request = capabilities::local_map().clone();
        request.extend(
            parameters
                .into_iter()
                .map(|(k, v)| (k, Dynamic(v.into_owned()))),
        );
        let mut authenticate_result = client
            .call(
                AUTHENTICATE_ADDRESS,
                T::serialize(&request).map_err(Into::into)?,
            )
            .await?;
        let mut shared_capabilities = Deserialize::deserialize(authenticate_result.deserializer())
            .map_err(Into::into)
            .map_err(Into::into)?;
        authentication::extract_state_result(&mut shared_capabilities)?;
        capabilities::check_required(&shared_capabilities)?;
        self.capabilities.send_replace(Some(shared_capabilities));
        Ok(())
    }
}

impl std::fmt::Debug for Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Control")
            .field("capabilities", &self.capabilities)
            .field("remote_authorized", &self.remote_authorized)
            .finish()
    }
}

pub(super) struct Control<H> {
    pub(super) controller: Arc<Controller>,
    pub(super) capabilities: watch::Receiver<Option<CapabilitiesMap<'static>>>,
    pub(super) remote_authorized: watch::Receiver<bool>,
    pub(super) handler: ControlledHandler<H>,
}

pub(super) fn make<Handler, Auth, In, Out>(
    handler: Handler,
    authenticator: Auth,
    remote_authorized: bool,
) -> Control<Handler>
where
    Handler: messaging::Handler<In, Reply = Out, Error = Error>,
    Auth: Authenticator + Send + Sync + 'static,
    In: messaging::BodyBuf<Error = Error>,
    Out: messaging::BodyBuf<Error = Error>,
{
    let (capabilities_sender, capabilities_receiver) = watch::channel(Default::default());
    let (remote_authorized_sender, remote_authorized_receiver) = watch::channel(remote_authorized);
    let controller = Arc::new(Controller {
        authenticator: Box::new(authenticator),
        capabilities: capabilities_sender,
        remote_authorized: remote_authorized_sender,
    });
    let controlled_handler = ControlledHandler {
        inner: handler,
        controller: Arc::clone(&controller),
    };
    Control {
        controller,
        capabilities: capabilities_receiver,
        remote_authorized: remote_authorized_receiver,
        handler: controlled_handler,
    }
}

pub(super) struct ControlledHandler<H> {
    inner: H,
    controller: Arc<Controller>,
}

impl<S, In, Out> messaging::Handler<In> for ControlledHandler<S>
where
    S: messaging::Handler<In, Reply = Out, Error = Error> + Sync,
    In: messaging::BodyBuf<Error = Error> + Send,
    for<'de> <In::Deserializer<'de> as serde::Deserializer<'de>>::Error: Into<In::Error>,
    Out: messaging::BodyBuf<Error = Error> + Send,
{
    type Reply = Out;
    type Error = Error;

    fn call(
        &self,
        address: message::Address,
        mut value: In,
    ) -> impl Future<Output = Result<Out>> + Send {
        if is_control_address(address) {
            let controller = Arc::clone(&self.controller);
            future::ready(
                Deserialize::deserialize(value.deserializer())
                    .map_err(Into::into)
                    .and_then(|request| controller.authenticate(request))
                    .and_then(|result| Out::serialize(&result)),
            )
            .left_future()
        } else if *self.controller.remote_authorized.borrow() {
            self.inner.call(address, value).err_into().right_future()
        } else {
            future::err(Error::NoMessageHandler(message::Type::Call, address)).left_future()
        }
    }

    async fn oneway(&self, address: message::Address, request: message::Oneway<In>) -> Result<()> {
        if is_control_address(address) {
            // TODO: Handle capabilities request ?
            Ok(())
        } else if *self.controller.remote_authorized.borrow() {
            self.inner.oneway(address, request).await
        } else {
            Err(Error::NoMessageHandler(request.ty(), address))
        }
    }
}
