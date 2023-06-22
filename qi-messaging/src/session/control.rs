mod authentication;
pub(super) mod capabilities;

use self::authentication::authenticate;
use crate::{
    channel::RequestIdSequence, format, messaging, Bytes, Client, IsErrorCanceledTermination,
};
use capabilities::{CapabilitiesMap, CapabilitiesMapExt};
use futures::{future, FutureExt, TryFutureExt};
use std::{future::Future, sync::Arc};
use tokio::sync::{watch, Mutex};
use tracing::{debug, instrument};

mod subject {
    use crate::messaging;
    pub(super) use crate::messaging::subject::{Action, Object, Service};

    const CONTROL_SERVICE: Service = Service::new(0);
    const CONTROL_OBJECT: Object = Object::new(0);

    pub(crate) fn is_service(service: Service) -> bool {
        service == CONTROL_SERVICE
    }

    pub(crate) fn is_object(object: Object) -> bool {
        object == CONTROL_OBJECT
    }

    pub(super) fn is_subject(subject: messaging::Subject) -> bool {
        is_service(subject.service()) && is_object(subject.object())
    }

    #[derive(
        Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
    )]
    pub(crate) struct Subject(pub(super) Action);

    impl Subject {
        pub(super) fn from_message(subject: messaging::Subject) -> Option<Self> {
            is_subject(subject).then_some(Self(subject.action()))
        }
    }

    impl From<Subject> for messaging::Subject {
        fn from(subject: Subject) -> Self {
            Self::new(CONTROL_SERVICE, CONTROL_OBJECT, subject.0)
        }
    }

    impl PartialEq<messaging::Subject> for Subject {
        fn eq(&self, other: &messaging::Subject) -> bool {
            is_subject(*other) && other.action() == self.0
        }
    }

    impl PartialEq<Subject> for messaging::Subject {
        fn eq(&self, other: &Subject) -> bool {
            other == self
        }
    }
}
pub(super) use subject::{is_object, is_service, Subject};

pub(super) fn create() -> (Control, Service) {
    let capabilities = Arc::new(Mutex::new(CapabilitiesMap::new()));
    let (remote_authenticated_sender, remote_authenticated_receiver) = watch::channel(false);
    (
        Control {
            capabilities: Arc::clone(&capabilities),
            remote_authentication_receiver: remote_authenticated_receiver,
        },
        Service {
            capabilities,
            remote_authentication_sender: remote_authenticated_sender,
        },
    )
}

#[derive(Debug)]
pub(super) struct Control {
    capabilities: Arc<Mutex<CapabilitiesMap>>,
    remote_authentication_receiver: watch::Receiver<bool>,
}

impl Control {
    #[instrument(name = "authenticate", level = "debug", skip_all, ret)]
    pub(super) async fn authenticate_to_remote(
        &self,
        client: &mut Client,
        id_sequence: &RequestIdSequence,
    ) -> Result<(), AuthenticateToRemoteError> {
        use crate::service::Service;
        let authenticate = Authenticate::new_outgoing();
        let call = authenticate
            .to_messaging_call()
            .map_err(AuthenticateToRemoteError::SerializeLocalCapabilities)?;
        debug!("sending authentication request to server");
        let reply = client.call(id_sequence.pair_with_new_id(call)).await?;
        let result_capabilities = format::from_bytes(&reply)
            .map_err(AuthenticateToRemoteError::DeserializeRemoteCapabilities)?;
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
    pub(super) async fn remote_authentication(&mut self) -> Result<(), RemoteAuthenticationError> {
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
}

#[derive(Debug, thiserror::Error)]
pub(super) enum AuthenticateToRemoteError {
    #[error("request error")]
    SendRequest(#[from] crate::client::Error),

    #[error("error serializing local capabilities")]
    SerializeLocalCapabilities(#[source] format::Error),

    #[error("error deserializing remote capabilities")]
    DeserializeRemoteCapabilities(#[source] format::Error),

    #[error("error verifying the authentication result")]
    VerifyAuthenticationResult(#[from] VerifyAuthenticationResultError),

    #[error("some required capabilities are missing")]
    MissingRequiredCapabilities(#[from] capabilities::ExpectedKeyValueError<bool>),
}

pub(super) use authentication::VerifyResultError as VerifyAuthenticationResultError;

#[derive(Debug, thiserror::Error)]
pub(super) enum RemoteAuthenticationError {
    #[error("control service closed")]
    ServiceClosed,
}

#[derive(Debug)]
pub(super) struct Service {
    capabilities: Arc<Mutex<CapabilitiesMap>>,
    remote_authentication_sender: watch::Sender<bool>,
}

impl Service {
    fn authenticate(&self, parameters: &CapabilitiesMap) -> CapabilitiesMap {
        let reply = authenticate(parameters);
        self.remote_authentication_sender.send_replace(true);
        reply
    }

    fn update_capabilities(
        &self,
        remote: CapabilitiesMap,
    ) -> impl Future<Output = Result<(), UpdateCapabilitiesError>> {
        let check_result = remote.check_intersect_with_local();
        let self_capabilities = Arc::clone(&self.capabilities);
        async move {
            match check_result {
                Ok(capabilities) => {
                    *self_capabilities.lock_owned().await = capabilities;
                    Ok(())
                }
                Err(err) => Err(UpdateCapabilitiesError(err)),
            }
        }
    }
}

impl crate::Service<Call, Notification> for Service {
    type Error = Error;
    type CallFuture = future::Ready<Result<Bytes, Error>>;
    type NotifyFuture = future::BoxFuture<'static, Result<(), Error>>;

    fn call(&mut self, call: Call) -> Self::CallFuture {
        match call {
            Call::Authenticate(Authenticate(parameters)) => {
                let auth_result = self.authenticate(&parameters);
                let reply_payload_result =
                    format::to_bytes(&auth_result).map_err(Error::SerializeAuthenticationResult);
                future::ready(reply_payload_result)
            }
        }
    }

    fn notify(&mut self, notif: Notification) -> Self::NotifyFuture {
        match notif {
            Notification::Capabilities(Capabilities(capabilities)) => self
                .update_capabilities(capabilities)
                .map_err(Error::Capabilities)
                .boxed(),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum Call {
    Authenticate(Authenticate),
}

impl Call {
    pub(super) fn from_messaging(call: &messaging::Call) -> Result<Option<Self>, format::Error> {
        Ok(match Subject::from_message(call.subject) {
            Some(Authenticate::SUBJECT) => {
                let capabilities = format::from_bytes(&call.payload)?;
                Some(Self::Authenticate(Authenticate(capabilities)))
            }
            Some(_) | None => None,
        })
    }
}

#[derive(Debug, Clone, derive_more::Into)]
pub(super) struct Authenticate(CapabilitiesMap);

impl Authenticate {
    const SUBJECT: Subject = Subject(subject::Action::new(8));

    pub(super) fn new_outgoing() -> Self {
        Self(capabilities::local().clone())
    }

    pub(super) fn to_messaging_call(&self) -> Result<messaging::Call, format::Error> {
        Ok(messaging::Call {
            subject: Self::SUBJECT.into(),
            payload: format::to_bytes(&self.0)?,
        })
    }
}

#[derive(Debug, Clone)]
pub(super) enum Notification {
    Capabilities(Capabilities),
}

impl Notification {
    pub(super) fn from_messaging(
        notif: messaging::Notification,
    ) -> Result<Self, messaging::Notification> {
        match notif {
            messaging::Notification::Capabilities(messaging::Capabilities {
                subject,
                capabilities,
            }) if subject == Capabilities::SUBJECT => {
                Ok(Self::Capabilities(Capabilities(capabilities)))
            }
            _ => Err(notif),
        }
    }
}

#[derive(Debug, Clone, derive_more::Into)]
pub(super) struct Capabilities(CapabilitiesMap);

impl Capabilities {
    const SUBJECT: Subject = Subject(subject::Action::new(0));
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("error during serialization of authentication result")]
    SerializeAuthenticationResult(#[source] format::Error),

    #[error(transparent)]
    Capabilities(#[from] UpdateCapabilitiesError),
}

#[derive(Debug, thiserror::Error)]
#[error("error updating capabilities")]
pub(super) struct UpdateCapabilitiesError(#[from] capabilities::ExpectedKeyValueError<bool>);

impl IsErrorCanceledTermination for Error {
    fn is_canceled(&self) -> bool {
        false
    }
}
