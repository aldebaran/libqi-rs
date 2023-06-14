mod authentication;
pub(super) mod capabilities;
pub(super) mod request;

use self::authentication::authenticate;
use crate::{
    channel, format,
    message::{self, Action},
    IsCanceledError,
};
use capabilities::{CapabilitiesMap, CapabilitiesMapExt};
use futures::{future, FutureExt};
pub(super) use request::Request;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::{watch, Mutex};
use tracing::{debug, instrument};

const CONTROL_SERVICE: message::Service = message::Service::new(0);
const CONTROL_OBJECT: message::Object = message::Object::new(0);

pub(super) fn is_control_service(service: message::Service) -> bool {
    service == CONTROL_SERVICE
}

pub(super) fn is_control_object(object: message::Object) -> bool {
    object == CONTROL_OBJECT
}

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
struct Subject(Action);

impl From<Subject> for message::Subject {
    fn from(subject: Subject) -> Self {
        Self::new(CONTROL_SERVICE, CONTROL_OBJECT, subject.0)
    }
}

impl PartialEq<message::Subject> for Subject {
    fn eq(&self, other: &message::Subject) -> bool {
        other.service() == CONTROL_SERVICE
            && other.object() == CONTROL_OBJECT
            && other.action() == self.0
    }
}

impl PartialEq<Subject> for message::Subject {
    fn eq(&self, other: &Subject) -> bool {
        other == self
    }
}

#[derive(Debug)]
pub(super) struct Control {
    capabilities: Arc<Mutex<CapabilitiesMap>>,
    remote_authentication_receiver: watch::Receiver<bool>,
}

impl Control {
    pub(in crate::session) fn new_with_service() -> (Self, Service) {
        let capabilities = Arc::new(Mutex::new(CapabilitiesMap::new()));
        let (remote_authenticated_sender, remote_authenticated_receiver) = watch::channel(false);
        (
            Self {
                capabilities: Arc::clone(&capabilities),
                remote_authentication_receiver: remote_authenticated_receiver,
            },
            Service {
                capabilities,
                remote_authentication_sender: remote_authenticated_sender,
            },
        )
    }

    #[instrument(name = "authenticate", level = "debug", skip_all, ret)]
    pub(in crate::session) async fn authenticate_to_remote(
        &self,
        channel: &mut channel::Channel,
    ) -> Result<(), AuthenticateToRemoteError> {
        use tower::Service;
        let authenticate_request = request::Authenticate::new();
        let channel_request: channel::as_service::Call = authenticate_request
            .try_into()
            .map_err(AuthenticateToRemoteError::SerializeLocalCapabilities)?;
        debug!("sending authentication request to server");
        let response = async move { channel.ready().await?.call(channel_request).await };
        let result_capabilities: CapabilitiesMap = match response.await {
            Ok(reply) => format::from_bytes(&reply)
                .map_err(AuthenticateToRemoteError::DeserializeRemoteCapabilities)?,
            Err(channel::Error::Call(err)) => return Err(AuthenticateToRemoteError::Service(err)),
            Err(channel::Error::Dispatch(err)) => {
                return Err(AuthenticateToRemoteError::Dispatch(err))
            }
        };
        debug!(capabilities = ?result_capabilities, "received authentication result and capabilities from server");
        authentication::verify_result(&result_capabilities)?;
        let capabilities = result_capabilities
            .check_intersect_with_local()
            .map_err(AuthenticateToRemoteError::MissingRequiredCapabilities)?;
        debug!(
            ?capabilities,
            "resolved capabilities between local and remote"
        );
        *self.capabilities.lock().await = capabilities;
        Ok(())
    }

    #[instrument(level = "debug", name = "authentication", skip_all, ret)]
    pub(in crate::session) async fn remote_authentication(
        &mut self,
    ) -> Result<(), RemoteAuthenticationError> {
        match self
            .remote_authentication_receiver
            .wait_for(|auth| {
                debug!(result = auth, "received remote authentication result");
                *auth
            })
            .await
        {
            Ok(_ref) => Ok(()),
            Err(_err) => Err(RemoteAuthenticationError::ServiceClosed),
        }
    }

    pub(in crate::session) async fn capabilities(&self) -> CapabilitiesMap {
        Arc::clone(&self.capabilities).lock_owned().await.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RemoteAuthenticationError {
    #[error("the remote failed to authenticate")]
    AuthenticationFailure,

    #[error("control service closed")]
    ServiceClosed,
}

#[derive(Debug)]
pub(super) struct Service {
    capabilities: Arc<Mutex<CapabilitiesMap>>,
    remote_authentication_sender: watch::Sender<bool>,
}

impl Service {
    fn authenticate_remote(&self, parameters: &CapabilitiesMap) -> CapabilitiesMap {
        let result: CapabilitiesMap = authenticate(parameters);
        self.remote_authentication_sender.send_replace(true);
        result
    }

    fn update_capabilities(
        &self,
        remote: &CapabilitiesMap,
    ) -> Result<impl std::future::Future<Output = ()>, UpdateCapabilitiesError> {
        let capabilities = remote.clone().check_intersect_with_local()?;
        let self_capabilities = Arc::clone(&self.capabilities);
        Ok(async move {
            *self_capabilities.lock_owned().await = capabilities;
        })
    }
}

impl tower::Service<Request> for Service {
    type Response = Option<CapabilitiesMap>;
    type Error = Error;
    type Future = future::BoxFuture<'static, Result<Option<CapabilitiesMap>, Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req {
            Request::Authenticate(authenticate) => {
                let result = self.authenticate_remote(authenticate.parameters());
                future::ok(Some(result)).boxed()
            }
            Request::UpdateCapabilities(update_capabilities) => {
                let res = self.update_capabilities(update_capabilities.remote_map());
                match res {
                    Ok(update_future) => update_future.map(|()| Ok(None)).boxed(),
                    Err(err) => future::err(Error::UpdateCapabilities(err)).boxed(),
                }
                .boxed()
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error(transparent)]
    AuthenticateRemote(#[from] AuthenticateRemoteError),

    #[error(transparent)]
    UpdateCapabilities(#[from] UpdateCapabilitiesError),
}

impl IsCanceledError for Error {
    fn is_canceled(&self) -> bool {
        false
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum AuthenticateRemoteError {}

#[derive(Debug, thiserror::Error)]
#[error("error updating capabilities")]
pub(super) struct UpdateCapabilitiesError(#[from] capabilities::ExpectedKeyValueError<bool>);

pub use authentication::VerifyResultError as VerifyAuthenticationResultError;

#[derive(Debug, thiserror::Error)]
pub enum AuthenticateToRemoteError {
    #[error("request dispatch error")]
    Dispatch(#[from] crate::client::DispatchError),

    #[error(transparent)]
    Service(#[from] crate::client::CallError),

    #[error("error verifying the authentication result")]
    VerifyAuthenticationResult(#[from] VerifyAuthenticationResultError),

    #[error("error serializing local capabilities")]
    SerializeLocalCapabilities(#[source] format::Error),

    #[error("error deserializing remote capabilities")]
    DeserializeRemoteCapabilities(#[source] format::Error),

    #[error("some required capabilities are missing")]
    MissingRequiredCapabilities(#[from] capabilities::ExpectedKeyValueError<bool>),
}
