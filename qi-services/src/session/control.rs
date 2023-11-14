mod authentication;
mod capabilities;

use futures::{future::BoxFuture, FutureExt};
use qi_messaging::{self as messaging, capabilities::CapabilitiesMap, message};
use qi_value::{ActionId, ObjectId, ServiceId};

use crate::error::Error;

pub(crate) const SERVICE_ID: ServiceId = ServiceId(0);
pub(crate) const OBJECT_ID: ObjectId = ObjectId(0);
pub(crate) const AUTHENTICATE_ACTION_ID: ActionId = ActionId(0);

#[derive(Clone, Debug)]
struct Client {
    client: messaging::Client,
}

impl tower::Service<authentication::Authenticate> for Client {
    type Response = ();
    type Error = Error;
    type Future = BoxFuture<'static, Result<(), Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        tower::Service::<messaging::Call<CapabilitiesMap>>::poll_ready(&mut self.client, cx)
            .map_err(Into::into)
    }

    fn call(&mut self, auth: authentication::Authenticate) -> Self::Future {
        let call = self.client.call(messaging::Call::new(
            message::Address::new(SERVICE_ID, OBJECT_ID, AUTHENTICATE_ACTION_ID),
            auth.capabilities,
        ));
        async move {
            let reply = call.await?;
            Ok(())
        }
        .boxed()
    }
}

#[derive(Debug)]
struct Control {}

// fn update_capabilities(
//     &self,
//     remote: CapabilitiesMap,
// ) -> impl Future<Output = Result<(), UpdateCapabilitiesError>> {
//     let check_result = remote.check_intersect_with_local();
//     let self_capabilities = Arc::clone(&self.capabilities);
//     async move {
//         match check_result {
//             Ok(capabilities) => {
//                 *self_capabilities.lock_owned().await = capabilities;
//                 Ok(())
//             }
//             Err(err) => Err(UpdateCapabilitiesError(err)),
//         }
//     }
// }

//     capabilities: Arc<Mutex<CapabilitiesMap>>,
//     remote_authentication_receiver: watch::Receiver<bool>,
// }
// async fn authenticate(&self, params: CapabilitiesMap) -> Result<(), AuthenticateToRemoteError> {
//     use crate::service::Service;
//     let authenticate = Authenticate::new_outgoing();
//     let call = authenticate
//         .to_messaging_call()
//         .map_err(AuthenticateToRemoteError::SerializeLocalCapabilities)?;
//     trace!("sending authentication request to server");
//     let reply = client.call(call).await?;
//     let result_capabilities = reply
//         .value()
//         .map_err(AuthenticateToRemoteError::DeserializeRemoteCapabilities)?;
//     trace!(capabilities = ?result_capabilities, "received authentication result and capabilities from server");
//     authentication::verify_result(&result_capabilities)?;
//     let capabilities = result_capabilities
//         .check_intersect_with_local()
//         .map_err(AuthenticateToRemoteError::MissingRequiredCapabilities)?;
//     trace!(
//         ?capabilities,
//         "resolved capabilities between local and remote"
//     );
//     *self.capabilities.lock().await = capabilities;
//     Ok(())
// }
// impl Control {
//     #[instrument(name = "authenticate", level = "trace", skip_all, ret)]
//     pub(super) async fn authenticate_to_remote(
//         &self,
//         client: &mut client::Client,
//     ) -> Result<(), AuthenticateToRemoteError> {
//         use crate::service::Service;
//         let authenticate = Authenticate::new_outgoing();
//         let call = authenticate
//             .to_messaging_call()
//             .map_err(AuthenticateToRemoteError::SerializeLocalCapabilities)?;
//         trace!("sending authentication request to server");
//         let reply = client.call(call).await?;
//         let result_capabilities = reply
//             .value()
//             .map_err(AuthenticateToRemoteError::DeserializeRemoteCapabilities)?;
//         trace!(capabilities = ?result_capabilities, "received authentication result and capabilities from server");
//         authentication::verify_result(&result_capabilities)?;
//         let capabilities = result_capabilities
//             .check_intersect_with_local()
//             .map_err(AuthenticateToRemoteError::MissingRequiredCapabilities)?;
//         trace!(
//             ?capabilities,
//             "resolved capabilities between local and remote"
//         );
//         *self.capabilities.lock().await = capabilities;
//         Ok(())
//     }

//     #[instrument(name = "authentication", level = "trace", skip_all, ret)]
//     pub(super) async fn remote_authentication(&mut self) -> Result<(), RemoteAuthenticationError> {
//         match self
//             .remote_authentication_receiver
//             .wait_for(|auth| {
//                 trace!(result = auth, "received remote authentication result");
//                 *auth
//             })
//             .await
//         {
//             Ok(_ref) => Ok(()),
//             Err(_err) => Err(RemoteAuthenticationError::ServiceClosed),
//         }
//     }
// }

// #[derive(Debug, thiserror::Error)]
// pub(super) enum AuthenticateToRemoteError {
//     #[error(transparent)]
//     Client(#[from] client::Error),

//     #[error("the authentication request was canceled")]
//     Canceled,

//     #[error("error serializing local capabilities")]
//     SerializeLocalCapabilities(#[source] format::Error),

//     #[error("error deserializing remote capabilities")]
//     DeserializeRemoteCapabilities(#[source] format::Error),

//     #[error("error verifying the authentication result")]
//     VerifyAuthenticationResult(#[from] VerifyAuthenticationResultError),

//     #[error("some required capabilities are missing")]
//     MissingRequiredCapabilities(#[from] capabilities::ExpectedKeyValueError<bool>),
// }

// impl From<CallTermination<client::Error>> for AuthenticateToRemoteError {
//     fn from(value: CallTermination<client::Error>) -> Self {
//         match value {
//             CallTermination::Canceled => Self::Canceled,
//             CallTermination::Error(err) => Self::Client(err),
//         }
//     }
// }

// pub(super) use authentication::VerifyResultError as VerifyAuthenticationResultError;

// #[derive(Debug, thiserror::Error)]
// pub(super) enum RemoteAuthenticationError {
//     #[error("control service closed")]
//     ServiceClosed,
// }

// #[derive(Debug)]
// pub(super) struct Service {
//     capabilities: Arc<Mutex<CapabilitiesMap>>,
//     remote_authentication_sender: watch::Sender<bool>,
// }

// impl Service {
//     fn authenticate(&self, parameters: &CapabilitiesMap) -> CapabilitiesMap {
//         let reply = authenticate(parameters);
//         self.remote_authentication_sender.send_replace(true);
//         reply
//     }

//     fn update_capabilities(
//         &self,
//         remote: CapabilitiesMap,
//     ) -> impl Future<Output = Result<(), UpdateCapabilitiesError>> {
//         let check_result = remote.check_intersect_with_local();
//         let self_capabilities = Arc::clone(&self.capabilities);
//         async move {
//             match check_result {
//                 Ok(capabilities) => {
//                     *self_capabilities.lock_owned().await = capabilities;
//                     Ok(())
//                 }
//                 Err(err) => Err(UpdateCapabilitiesError(err)),
//             }
//         }
//     }
// }

// impl crate::Service<Call, Notification> for Service {
//     type CallReply = CapabilitiesMap;
//     type Error = Error;
//     type CallFuture = future::Ready<CallResult<Self::CallReply, Self::Error>>;
//     type NotifyFuture = future::BoxFuture<'static, Result<(), Self::Error>>;

//     fn call(&mut self, call: Call) -> Self::CallFuture {
//         match call {
//             Call::Authenticate(Authenticate(parameters)) => {
//                 future::ok(self.authenticate(&parameters))
//             }
//         }
//     }

//     fn notify(&mut self, notif: Notification) -> Self::NotifyFuture {
//         match notif {
//             Notification::Capabilities(Capabilities(capabilities)) => self
//                 .update_capabilities(capabilities)
//                 .map_err(Error::Capabilities)
//                 .boxed(),
//         }
//     }
// }

// #[derive(Debug, Clone)]
// pub(super) enum Call {
//     Authenticate(Authenticate),
// }

// impl Call {
//     pub(super) fn from_messaging(call: &messaging::Call) -> Result<Option<Self>, format::Error> {
//         Ok(match Subject::from_message(*call.subject()) {
//             Some(Authenticate::SUBJECT) => {
//                 let capabilities = call.value()?;
//                 Some(Self::Authenticate(Authenticate(capabilities)))
//             }
//             Some(_) | None => None,
//         })
//     }
// }

// #[derive(Debug, Clone, derive_more::Into)]
// pub(super) struct Authenticate(CapabilitiesMap);

// impl Authenticate {
//     const SUBJECT: Subject = Subject(ActionId::new(8));

//     pub(super) fn new_outgoing() -> Self {
//         Self(capabilities::local().clone())
//     }

//     pub(super) fn to_messaging_call(&self) -> Result<messaging::Call, format::Error> {
//         messaging::Call::new(Self::SUBJECT.into()).with_value(&self.0)
//     }
// }

// #[derive(Debug, Clone)]
// pub(super) enum Notification {
//     Capabilities(Capabilities),
// }

// impl Notification {
//     pub(super) fn from_messaging(
//         notif: messaging::Notification,
//     ) -> Result<Self, messaging::Notification> {
//         match notif {
//             messaging::Notification::Capabilities(capabilities_notif)
//                 if capabilities_notif.subject() == &Capabilities::SUBJECT =>
//             {
//                 Ok(Self::Capabilities(Capabilities(capabilities_notif.into())))
//             }
//             _ => Err(notif),
//         }
//     }
// }

// #[derive(Debug, Clone, derive_more::Into)]
// pub(super) struct Capabilities(CapabilitiesMap);

// impl Capabilities {
//     const SUBJECT: Subject = Subject(ActionId::new(0));
// }

// #[derive(Debug, thiserror::Error)]
// pub(super) enum Error {
//     #[error(transparent)]
//     Capabilities(#[from] UpdateCapabilitiesError),
// }

// #[derive(Debug, thiserror::Error)]
// #[error("error updating capabilities")]
// pub(super) struct UpdateCapabilitiesError(#[from] capabilities::ExpectedKeyValueError<bool>);
