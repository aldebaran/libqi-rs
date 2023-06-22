mod control;
mod router;

pub use crate::RequestId;
use crate::{
    channel, client, messaging,
    service::{self, CallTermination, ToSubject, WithRequestId},
    Bytes, IsErrorCanceledTermination, Service,
};
use futures::{FutureExt, TryFutureExt};
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct Client {
    client: client::Client,
    id_sequence: channel::RequestIdSequence,
}

impl crate::Service<Call, Notification> for Client {
    type Error = ClientError;
    type CallFuture = CallFuture;
    type NotifyFuture = NotifyFuture;

    fn call(&mut self, call: Call) -> Self::CallFuture {
        let call = self.id_sequence.pair_with_new_id(call.into());
        CallFuture(self.client.call(call))
    }

    fn notify(&mut self, notif: Notification) -> Self::NotifyFuture {
        let notif = self.id_sequence.pair_with_new_id(notif.into());
        NotifyFuture(self.client.notify(notif))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ClientError(#[from] client::Error);

pub fn connect<IO, Svc>(
    io: IO,
    service: Svc,
) -> (
    impl Future<Output = Result<Client, ConnectError>>,
    impl Future<Output = Result<(), Error>>,
)
where
    IO: AsyncWrite + AsyncRead,
    Svc: Service<CallWithId, NotificationWithId>,
    Svc::Error:
        IsErrorCanceledTermination + std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    // As a client, we can enable the service in the router right away.
    let (control, control_service) = control::create();
    let router = router::Router::with_service_enabled(control_service, service);
    let (mut client, id_sequence, channel_dispatch) = channel::open(io, router);

    let client = async move {
        control
            .authenticate_to_remote(&mut client, &id_sequence)
            .await?;
        Ok(Client {
            client,
            id_sequence,
        })
    };
    let session = channel_dispatch.map_err(|err| Error(err.into()));

    (client, session)
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("the server returned an authentication error: {0}")]
    AuthenticationFailure(String),

    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl From<control::AuthenticateToRemoteError> for ConnectError {
    fn from(error: control::AuthenticateToRemoteError) -> Self {
        use control::AuthenticateToRemoteError as AuthError;
        use control::VerifyAuthenticationResultError;
        match error {
            AuthError::SendRequest(client::Error::Terminated(CallTermination::Error(message)))
            | AuthError::VerifyAuthenticationResult(VerifyAuthenticationResultError::Error(
                message,
            )) => Self::AuthenticationFailure(message),
            _ => Self::Other(error.into()),
        }
    }
}

pub fn listen<IO, Svc>(
    io: IO,
    service: Svc,
) -> (
    impl Future<Output = Result<Client, ListenError>>,
    impl Future<Output = Result<(), Error>>,
)
where
    IO: AsyncWrite + AsyncRead + Send + 'static,
    Svc: Service<CallWithId, NotificationWithId>,
    Svc::Error:
        IsErrorCanceledTermination + std::fmt::Display + std::fmt::Debug + Sync + Send + 'static,
{
    // As a server, we first have to create the router, then wait for a successful
    // authentication to enable access to the service.

    let (mut control, control_service) = control::create();
    let (router, router_enable_service_sender) = router::Router::new(control_service);
    let (client, id_sequence, channel_dispatch) = channel::open(io, router);

    let client = async move {
        control.remote_authentication().await?;
        if router_enable_service_sender
            .send(router::EnableService::new(service))
            .is_err()
        {
            debug!("failed to enable the service of the session router, the router service is probably terminated.");
        }
        Ok(Client {
            client,
            id_sequence,
        })
    };
    let session = channel_dispatch.map_err(|err| Error(err.into()));

    (client, session)
}

#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("the connection was terminated")]
    Terminated(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl From<control::RemoteAuthenticationError> for ListenError {
    fn from(error: control::RemoteAuthenticationError) -> Self {
        Self::Terminated(error.into())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(#[from] Box<dyn std::error::Error + Send + Sync>);

pub mod subject {
    pub use crate::message::{Action, Object, Service};
    use crate::{message, session::control};

    #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub struct Subject {
        service: Service,
        object: Object,
        action: Action,
    }

    impl Subject {
        pub fn new(service: Service, object: Object, action: Action) -> Option<Self> {
            if control::is_service(service) || control::is_object(object) {
                None
            } else {
                Some(Self {
                    service,
                    object,
                    action,
                })
            }
        }

        pub fn service(&self) -> Service {
            self.service
        }

        pub fn object(&self) -> Object {
            self.object
        }

        pub fn action(&self) -> Action {
            self.action
        }

        pub(crate) fn from_messaging(subject: message::Subject) -> Option<Self> {
            Self::new(subject.service(), subject.object(), subject.action())
        }

        pub(crate) fn into_messaging(self) -> message::Subject {
            message::Subject::new(self.service, self.object, self.action)
        }
    }

    impl From<Subject> for message::Subject {
        fn from(subject: Subject) -> Self {
            subject.into_messaging()
        }
    }
}

pub use subject::Subject;

pub type Request = service::Request<Call, Notification>;
pub type Call = service::Call<Subject>;

impl From<Call> for messaging::Call {
    fn from(call: Call) -> Self {
        Self {
            subject: call.subject.into(),
            payload: call.payload,
        }
    }
}

impl CallWithId {
    fn from_messaging(call: messaging::CallWithId) -> Result<Self, messaging::CallWithId> {
        match Subject::from_messaging(call.to_subject()) {
            Some(subject) => {
                let messaging::CallWithId {
                    id,
                    inner: messaging::Call { payload, .. },
                } = call;
                Ok(Self {
                    id,
                    inner: Call { subject, payload },
                })
            }
            None => Err(call),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Notification {
    Post(Post),
    Event(Event),
    Cancel(Cancel),
}

impl ToSubject for Notification {
    type Subject = Subject;

    fn to_subject(&self) -> Self::Subject {
        match self {
            Self::Post(post) => post.to_subject(),
            Self::Event(event) => event.to_subject(),
            Self::Cancel(cancel) => cancel.to_subject(),
        }
    }
}

impl From<Notification> for messaging::Notification {
    fn from(notif: Notification) -> Self {
        match notif {
            Notification::Post(Post { subject, payload }) => messaging::Post {
                subject: subject.into(),
                payload,
            }
            .into(),
            Notification::Event(Event { subject, payload }) => messaging::Event {
                subject: subject.into(),
                payload,
            }
            .into(),
            Notification::Cancel(Cancel { subject, call_id }) => messaging::Cancel {
                subject: subject.into(),
                call_id,
            }
            .into(),
        }
    }
}

impl NotificationWithId {
    fn from_messaging(
        notif: messaging::NotificationWithId,
    ) -> Result<Self, messaging::NotificationWithId> {
        let subject = match Subject::from_messaging(notif.to_subject()) {
            Some(subject) => subject,
            None => return Err(notif),
        };
        let id = notif.id;
        let notif_res = match notif.inner {
            messaging::Notification::Post(messaging::Post { payload, .. }) => {
                Ok(Notification::Post(Post { subject, payload }))
            }
            messaging::Notification::Event(messaging::Event { payload, .. }) => {
                Ok(Notification::Event(Event { subject, payload }))
            }
            messaging::Notification::Cancel(messaging::Cancel { call_id, .. }) => {
                Ok(Notification::Cancel(Cancel { subject, call_id }))
            }
            messaging::Notification::Capabilities(_) => Err(notif),
        };

        notif_res.map(|notif| WithRequestId { id, inner: notif })
    }
}

pub type Post = service::Post<Subject>;
pub type Event = service::Event<Subject>;
pub type Cancel = service::Cancel<Subject>;

pub type CallWithId = service::CallWithId<Subject>;
pub type NotificationWithId = WithRequestId<Notification>;

pub type PostWithId = service::PostWithId<Subject>;
pub type EventWithId = service::EventWithId<Subject>;
pub type CancelWithId = service::CancelWithId<Subject>;

#[derive(Debug, derive_more::From)]
#[must_use = "futures do nothing until polled"]
pub struct CallFuture(client::CallFuture);

impl Future for CallFuture {
    type Output = Result<Bytes, ClientError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx).map_err(Into::into)
    }
}

impl service::ToRequestId for CallFuture {
    fn to_request_id(&self) -> RequestId {
        self.0.to_request_id()
    }
}

#[derive(Debug, derive_more::From)]
#[must_use = "futures do nothing until polled"]
pub struct NotifyFuture(client::NotifyFuture);

impl Future for NotifyFuture {
    type Output = Result<(), ClientError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx).map_err(Into::into)
    }
}

impl service::ToRequestId for NotifyFuture {
    fn to_request_id(&self) -> RequestId {
        self.0.to_request_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::from_bytes;
    use futures::{
        future::{self, BoxFuture},
        FutureExt,
    };
    use qi_format::to_bytes;
    use tokio::{io, join, select, spawn};

    struct ServiceFn<T, U, E> {
        f: Box<dyn FnMut(T) -> BoxFuture<'static, Result<U, E>> + Send>,
    }

    impl<T, U, E> ServiceFn<T, U, E> {
        fn new<F, Fut>(mut f: F) -> Self
        where
            F: FnMut(T) -> Fut + Send + 'static,
            Fut: Future<Output = Result<U, E>> + Send + 'static,
        {
            Self {
                f: Box::new(move |input| f(input).boxed()),
            }
        }
    }

    impl<T, U, E> crate::Service<CallWithId, NotificationWithId> for ServiceFn<T, U, E>
    where
        T: serde::de::DeserializeOwned,
        U: serde::Serialize + Send + 'static,
        E: std::error::Error + Sync + Send + 'static,
    {
        type Error = Box<dyn std::error::Error + Sync + Send>;
        type CallFuture = BoxFuture<'static, Result<Bytes, Self::Error>>;
        type NotifyFuture = BoxFuture<'static, Result<(), Self::Error>>;

        fn call(&mut self, call: CallWithId) -> Self::CallFuture {
            let input = match from_bytes(&call.inner.payload) {
                Ok(input) => input,
                Err(err) => return future::err(err.into()).boxed(),
            };
            let output_future = (self.f)(input);
            async move {
                let output = output_future.await?;
                let output_bytes = to_bytes(&output)?;
                Ok(output_bytes)
            }
            .boxed()
        }

        fn notify(&mut self, _notif: NotificationWithId) -> Self::NotifyFuture {
            unimplemented!()
        }
    }

    fn to_async<F, T, U>(f: F) -> impl Fn(T) -> future::Ready<U>
    where
        F: Fn(T) -> U,
    {
        move |input| future::ready(f(input))
    }

    fn to_try<F, T, U>(f: F) -> impl Fn(T) -> Result<U, std::convert::Infallible>
    where
        F: Fn(T) -> U,
    {
        move |input| Ok(f(input))
    }

    fn add_to_string((a, b): (i32, i32)) -> String {
        (a + b).to_string()
    }

    fn sum(elems: Vec<i32>) -> i32 {
        elems.iter().sum()
    }

    struct TestSessionPair {
        client: super::Client,
        server: super::Client,
    }

    impl TestSessionPair {
        async fn new() -> Self {
            let (io_client, io_server) = io::duplex(256);
            let client_service = ServiceFn::new(to_async(to_try(sum)));
            let (client, client_dispatch) = connect(io_client, client_service);
            let server_service = ServiceFn::new(to_async(to_try(add_to_string)));
            let (server, server_dispatch) = listen(io_server, server_service);
            spawn(async move {
                select! {
                    res = client_dispatch => {
                        res.unwrap();
                    },
                    res = server_dispatch => {
                        res.unwrap();
                    }
                }
            });
            let (client, server) = join!(client.map(Result::unwrap), server.map(Result::unwrap));
            Self { client, server }
        }
    }

    fn any_service_subject() -> super::Subject {
        super::Subject::new(
            super::subject::Service::new(1),
            super::subject::Object::new(1),
            super::subject::Action::new(1),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_session_pair_call() {
        let TestSessionPair {
            mut client,
            mut server,
        } = TestSessionPair::new().await;

        let subject = any_service_subject();
        let reply = client
            .call(Call::with_content(subject, &(12, -49)).unwrap())
            .await
            .unwrap();
        let value: String = from_bytes(&reply).unwrap();
        assert_eq!(value, "-37");

        let reply = server
            .call(Call::with_content(subject, &vec![32, 2893, -123, 3287, 0, -38293]).unwrap())
            .await
            .unwrap();
        let value: i32 = from_bytes(&reply).unwrap();
        assert_eq!(value, -32204);
    }
}
