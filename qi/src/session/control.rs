use super::{
    authentication::{self, Authenticator},
    capabilities,
};
use crate::{
    error::{FormatError, HandlerError, NoHandlerError},
    messaging,
    value::{ActionId, KeyDynValueMap, ObjectId, ServiceId},
    Error,
};
use messaging::message;
use std::sync::Arc;
use tokio::sync::watch;

const SERVICE_ID: ServiceId = ServiceId(0);
const OBJECT_ID: ObjectId = ObjectId(0);
const AUTHENTICATE_ACTION_ID: ActionId = ActionId(8);

fn is_control_address(address: message::Address) -> bool {
    address.service() == SERVICE_ID && address.object() == OBJECT_ID
}

pub const AUTHENTICATE_ADDRESS: message::Address =
    message::Address(SERVICE_ID, OBJECT_ID, AUTHENTICATE_ACTION_ID);

#[derive(Clone)]
pub(super) struct Controller {
    authenticator: Arc<dyn Authenticator + Send + Sync>,
    capabilities: watch::Sender<Option<KeyDynValueMap>>,
    remote_authorized: watch::Sender<bool>,
}

impl Controller {
    fn authenticate(
        &self,
        request: KeyDynValueMap,
    ) -> Result<KeyDynValueMap, AuthenticateClientError> {
        let shared_capabilities = capabilities::shared_with_local(&request);
        capabilities::check_required(&shared_capabilities)?;
        self.authenticator
            .verify(request)
            .map_err(AuthenticateClientError::AuthenticationVerification)?;
        self.capabilities
            .send_replace(Some(shared_capabilities.clone()));
        self.remote_authorized.send_replace(true);
        Ok(authentication::state_done_map(shared_capabilities))
    }

    pub(super) async fn authenticate_to_server<Body>(
        &self,
        client: &mut messaging::Client<Body>,
        parameters: KeyDynValueMap,
    ) -> Result<(), Error>
    where
        Body: messaging::Body + Send,
        Body::Error: Send + Sync + 'static,
    {
        self.capabilities.send_replace(None); // Reset the capabilities
        let mut request = capabilities::local_map().clone();
        request.extend(parameters);
        let authenticate_result = client
            .call(
                AUTHENTICATE_ADDRESS,
                Body::serialize(&request).map_err(FormatError::ArgumentsSerialization)?,
            )
            .await?;
        let mut shared_capabilities = authenticate_result
            .deserialize()
            .map_err(FormatError::MethodReturnValueDeserialization)?;
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
    pub(super) controller: Controller,
    pub(super) capabilities: watch::Receiver<Option<KeyDynValueMap>>,
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
    let controller = Controller {
        authenticator: Arc::new(authenticator),
        capabilities: capabilities_sender,
        remote_authorized: remote_authorized_sender,
    };
    let controlled_handler = ControlledHandler {
        inner: handler,
        controller: controller.clone(),
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
    controller: Controller,
}

impl<Handler, Body> messaging::Handler<Body> for ControlledHandler<Handler>
where
    Handler: messaging::Handler<Body, Error = HandlerError> + Sync,
    Body: messaging::Body + Send,
    Body::Error: Send + Sync + 'static,
{
    type Error = HandlerError;

    async fn call(&self, address: message::Address, value: Body) -> Result<Body, Self::Error> {
        if is_control_address(address) {
            let request = value
                .deserialize()
                .map_err(FormatError::ArgumentsDeserialization)
                .map_err(HandlerError::non_fatal)?;
            let result = self
                .controller
                .authenticate(request)
                // All authentication errors are fatal
                .map_err(HandlerError::fatal)?;
            Body::serialize(&result)
                .map_err(FormatError::MethodReturnValueSerialization)
                .map_err(HandlerError::non_fatal)
        } else if *self.controller.remote_authorized.borrow() {
            self.inner.call(address, value).await
        } else {
            Err(HandlerError::non_fatal(NoHandlerError(
                message::Type::Call,
                address,
            )))
        }
    }

    async fn fire_and_forget(
        &self,
        address: message::Address,
        request: message::FireAndForget<Body>,
    ) {
        if is_control_address(address) {
            // TODO: Handle capabilities request ?
        } else if *self.controller.remote_authorized.borrow() {
            self.inner.fire_and_forget(address, request).await
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
