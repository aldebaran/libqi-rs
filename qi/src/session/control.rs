use super::{
    authentication::{self, Authenticator},
    capabilities,
};
use crate::{
    error::{FormatError, NoHandlerError},
    messaging::{self, CapabilitiesMap},
    Error,
};
use futures::{future, FutureExt, TryFutureExt};
use messaging::message;
use qi_messaging::OwnedCapabilitiesMap;
use qi_value::{ActionId, Dynamic, ObjectId, ServiceId, Value};
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
    fn authenticate(
        &self,
        request: CapabilitiesMap<'_>,
    ) -> Result<CapabilitiesMap<'static>, AuthenticateClientError> {
        let shared_capabilities = capabilities::shared_with_local(&request);
        capabilities::check_required(&shared_capabilities)?;
        let parameters = request.into_iter().map(|(k, v)| (k, v.0)).collect();
        self.authenticator
            .verify(parameters)
            .map_err(AuthenticateClientError::AuthenticationVerification)?;
        self.capabilities
            .send_replace(Some(shared_capabilities.clone()));
        self.remote_authorized.send_replace(true);
        Ok(authentication::state_done_map(shared_capabilities))
    }

    pub(super) async fn authenticate_to_server<Body>(
        &self,
        client: &mut messaging::Client<Body>,
        parameters: HashMap<String, Value<'_>>,
    ) -> Result<(), Error>
    where
        Body: messaging::Body + Send,
        Body::Error: Send + Sync + 'static,
    {
        self.capabilities.send_replace(None); // Reset the capabilities
        let mut request = capabilities::local_map().clone();
        request.extend(
            parameters
                .into_iter()
                .map(|(k, v)| (k, Dynamic(v.into_owned()))),
        );
        let authenticate_result = client
            .call(
                AUTHENTICATE_ADDRESS,
                Body::serialize(&request).map_err(FormatError::ArgumentsSerialization)?,
            )
            .await?;
        let mut shared_capabilities = authenticate_result
            .deserialize::<serde_with::de::DeserializeAsWrap<_, OwnedCapabilitiesMap>>()
            .map_err(FormatError::MethodReturnValueDeserialization)?
            .into_inner();
        authentication::extract_state_result(&mut shared_capabilities)
            .map_err(AuthenticateToServerError::ResultState)?;
        capabilities::check_required(&shared_capabilities)
            .map_err(AuthenticateToServerError::UnexpectedServerCapabilityValue)?;
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

pub(super) fn make<Handler, Auth, Body>(
    handler: Handler,
    authenticator: Auth,
    remote_authorized: bool,
) -> Control<Handler>
where
    Handler: messaging::Handler<Body>,
    Auth: Authenticator + Send + Sync + 'static,
    Body: messaging::Body,
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

impl<Handler, Body> messaging::Handler<Body> for ControlledHandler<Handler>
where
    Handler: messaging::Handler<Body> + Sync,
    Handler::Error: std::error::Error + Send + Sync + 'static,
    Body: messaging::Body + Send,
    Body::Error: Send + Sync + 'static,
{
    type Error = Error;

    fn call(
        &self,
        address: message::Address,
        value: Body,
    ) -> impl Future<Output = Result<Body, Self::Error>> + Send {
        if is_control_address(address) {
            let controller = Arc::clone(&self.controller);
            future::ready(
                value
                    .deserialize()
                    .map_err(FormatError::ArgumentsDeserialization)
                    .map_err(Into::into)
                    .and_then(move |request| controller.authenticate(request).map_err(Into::into))
                    .and_then(|result| {
                        Body::serialize(&result)
                            .map_err(FormatError::MethodReturnValueSerialization)
                            .map_err(Into::into)
                    }),
            )
            .left_future()
        } else if *self.controller.remote_authorized.borrow() {
            self.inner
                .call(address, value)
                .map_err(Into::into)
                .map_err(Error::Other)
                .right_future()
        } else {
            future::err(NoHandlerError(message::Type::Call, address).into()).left_future()
        }
    }

    async fn oneway(&self, address: message::Address, request: message::Oneway<Body>) {
        if is_control_address(address) {
            // TODO: Handle capabilities request ?
        } else if *self.controller.remote_authorized.borrow() {
            self.inner.oneway(address, request).await
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum AuthenticateClientError {
    #[error("unexpected capability value")]
    UnexpectedclientCapabilityValue(#[from] capabilities::KeyValueExpectError),

    #[error("failure to verify authentication request")]
    AuthenticationVerification(#[source] authentication::Error),
}

impl From<AuthenticateClientError> for Error {
    fn from(err: AuthenticateClientError) -> Self {
        Error::Other(err.into())
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum AuthenticateToServerError {
    #[error("the authentication state sent back by the server is invalid")]
    ResultState(#[from] authentication::StateError),

    #[error("the server sent an unexpected capability value")]
    UnexpectedServerCapabilityValue(#[from] capabilities::KeyValueExpectError),
}

impl From<AuthenticateToServerError> for Error {
    fn from(err: AuthenticateToServerError) -> Self {
        Error::Other(err.into())
    }
}
