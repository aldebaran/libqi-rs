mod control;
mod router;

use crate::{
    channel, client, messaging,
    service::{self, CallResult, GetSubject, WithRequestId},
    Service,
};
pub use crate::{client::CancelFuture, service::Reply, RequestId};
use futures::{FutureExt, TryFutureExt};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::trace;

#[derive(Debug, Clone)]
pub struct Client {
    client: client::Client,
}

impl crate::Service<Call, Notification> for Client {
    type CallReply = Reply;
    type Error = ClientError;
    type CallFuture = CallFuture;
    type NotifyFuture = NotifyFuture;

    fn call(&mut self, call: Call) -> Self::CallFuture {
        let mut this = &*self;
        this.call(call)
    }

    fn notify(&mut self, notif: Notification) -> Self::NotifyFuture {
        let mut this = &*self;
        this.notify(notif)
    }
}

impl crate::Service<Call, Notification> for &Client {
    type CallReply = Reply;
    type Error = ClientError;
    type CallFuture = CallFuture;
    type NotifyFuture = NotifyFuture;

    fn call(&mut self, call: Call) -> Self::CallFuture {
        let mut client = &self.client;
        CallFuture(client.call(call.into()))
    }

    fn notify(&mut self, notif: Notification) -> Self::NotifyFuture {
        let mut client = &self.client;
        NotifyFuture(client.notify(notif.into()))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum ClientError {
    #[error(transparent)]
    SessionClosed(#[from] SessionClosedError),

    // #[error("format serialization/deserialization error")]
    // Format(#[from] format::Error),
    #[error(transparent)]
    Service(#[from] service::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("session is closed")]
pub struct SessionClosedError(#[source] client::Error);

impl From<client::Error> for ClientError {
    fn from(error: client::Error) -> Self {
        match error {
            client::Error::DispatchTerminated => SessionClosedError(error).into(),
            client::Error::DispatchDroppedResponse => SessionClosedError(error).into(),
            client::Error::Messaging(err) => Self::Service(err),
        }
    }
}

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
    Svc::Error: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    Svc::CallReply: serde::Serialize,
{
    // As a client, we can enable the service in the router right away.
    let (control, control_service) = control::create();
    let router = router::Router::with_service_enabled(control_service, service);
    let (mut client, channel_dispatch) = channel::open(io, router);

    let client = async move {
        control.authenticate_to_remote(&mut client).await?;
        Ok(Client { client })
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
            AuthError::Client(client::Error::Messaging(messaging::Error(message)))
            | AuthError::VerifyAuthenticationResult(VerifyAuthenticationResultError::Refused(
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
    Svc::Error: std::fmt::Display + std::fmt::Debug + Sync + Send + 'static,
    Svc::CallReply: serde::Serialize,
{
    // As a server, we first have to create the router, then wait for a successful
    // authentication to enable access to the service.

    let (mut control, control_service) = control::create();
    let (router, router_enable_service_sender) = router::Router::new(control_service);
    let (client, channel_dispatch) = channel::open(io, router);

    let client = async move {
        control.remote_authentication().await?;
        if router_enable_service_sender
            .send(router::EnableService::new(service))
            .is_err()
        {
            trace!("failed to enable the service of the session router, the router service is probably terminated.");
        }
        Ok(Client { client })
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
    use crate::types::object::{ActionId, ObjectId, ServiceId};
    use crate::{message, session::control};

    #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub struct ServiceObject {
        service: ServiceId,
        object: ObjectId,
    }

    impl ServiceObject {
        pub fn new(service: ServiceId, object: ObjectId) -> Option<Self> {
            if control::is_service(service) || control::is_object(object) {
                None
            } else {
                Some(Self { service, object })
            }
        }

        pub fn service(&self) -> ServiceId {
            self.service
        }

        pub fn object(&self) -> ObjectId {
            self.object
        }
    }

    #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub struct Subject {
        service_object: ServiceObject,
        action: ActionId,
    }

    impl Subject {
        pub fn new(service_object: ServiceObject, action: ActionId) -> Self {
            Self {
                service_object,
                action,
            }
        }

        pub fn service(&self) -> ServiceId {
            self.service_object.service
        }

        pub fn object(&self) -> ObjectId {
            self.service_object.object
        }

        pub fn action(&self) -> ActionId {
            self.action
        }

        pub(crate) fn from_messaging(subject: message::Subject) -> Option<Self> {
            let service_object = ServiceObject::new(subject.service(), subject.object());
            service_object.map(|service_object| Self::new(service_object, subject.action()))
        }

        pub(crate) fn into_messaging(self) -> message::Subject {
            message::Subject::new(self.service(), self.object(), self.action())
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
        Self::new((*call.subject()).into()).with_formatted_value(call.into_formatted_value())
    }
}

impl CallWithId {
    fn from_messaging(call: messaging::CallWithId) -> Result<Self, messaging::CallWithId> {
        match Subject::from_messaging(*call.subject()) {
            Some(subject) => {
                let id = call.id();
                let call = Call::new(subject)
                    .with_formatted_value(call.into_inner().into_formatted_value());
                Ok(Self::new(id, call))
            }
            None => Err(call),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, derive_more::From)]
pub enum Notification {
    Post(Post),
    Event(Event),
    Cancel(Cancel),
}

impl GetSubject for Notification {
    type Subject = Subject;

    fn subject(&self) -> &Self::Subject {
        match self {
            Self::Post(post) => post.subject(),
            Self::Event(event) => event.subject(),
            Self::Cancel(cancel) => cancel.subject(),
        }
    }
}

impl From<Notification> for messaging::Notification {
    fn from(notif: Notification) -> Self {
        match notif {
            Notification::Post(post) => messaging::Post::from(post).into(),
            Notification::Event(event) => messaging::Event::from(event).into(),
            Notification::Cancel(cancel) => messaging::Cancel::from(cancel).into(),
        }
    }
}

impl NotificationWithId {
    fn from_messaging(
        notif: messaging::NotificationWithId,
    ) -> Result<Self, messaging::NotificationWithId> {
        let subject = match Subject::from_messaging(*notif.subject()) {
            Some(subject) => subject,
            None => return Err(notif),
        };
        let id = notif.id();
        let notif = match notif.into_inner() {
            messaging::Notification::Post(post) => Post::new(subject)
                .with_formatted_value(post.into_formatted_value())
                .into(),
            messaging::Notification::Event(event) => Event::new(subject)
                .with_formatted_value(event.into_formatted_value())
                .into(),
            messaging::Notification::Cancel(cancel) => {
                Cancel::new(subject, cancel.call_id()).into()
            }
            notif @ messaging::Notification::Capabilities(_) => {
                return Err(WithRequestId::new(id, notif))
            }
        };

        Ok(WithRequestId::new(id, notif))
    }
}

pub type Post = service::Post<Subject>;

impl From<Post> for messaging::Post {
    fn from(post: Post) -> Self {
        messaging::Post::new((*post.subject()).into())
    }
}

pub type Event = service::Event<Subject>;

impl From<Event> for messaging::Event {
    fn from(event: Event) -> Self {
        messaging::Event::new((*event.subject()).into())
    }
}

pub type Cancel = service::Cancel<Subject>;

impl From<Cancel> for messaging::Cancel {
    fn from(cancel: Cancel) -> Self {
        messaging::Cancel::new((*cancel.subject()).into(), cancel.call_id())
    }
}

pub type CallWithId = service::CallWithId<Subject>;
pub type NotificationWithId = WithRequestId<Notification>;

pub type PostWithId = service::PostWithId<Subject>;
pub type EventWithId = service::EventWithId<Subject>;
pub type CancelWithId = service::CancelWithId<Subject>;

#[derive(Debug, derive_more::From)]
#[must_use = "futures do nothing until polled"]
pub struct CallFuture(client::CallFuture);

impl CallFuture {
    pub fn cancel(mut self) -> CancelFuture {
        self.0.cancel()
    }
}

impl Future for CallFuture {
    type Output = CallResult<Reply, ClientError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_unpin(cx).map_err(|err| err.map_err(Into::into))
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

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
    use crate::{
        service::CallTermination,
        types::object::{ActionId, ObjectId, ServiceId},
    };
    use futures::{
        future::{self, BoxFuture},
        FutureExt,
    };
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
        type CallReply = U;
        type Error = Box<dyn std::error::Error + Sync + Send>;
        type CallFuture = BoxFuture<'static, CallResult<Self::CallReply, Self::Error>>;
        type NotifyFuture = BoxFuture<'static, Result<(), Self::Error>>;

        fn call(&mut self, call: CallWithId) -> Self::CallFuture {
            let input = match call.inner().value() {
                Ok(input) => input,
                Err(err) => return future::err(CallTermination::Error(err.into())).boxed(),
            };
            let output_future = (self.f)(input);
            async move {
                let output = output_future
                    .await
                    .map_err(|err| CallTermination::Error(err.into()))?;
                Ok(output)
            }
            .boxed()
        }

        fn notify(&mut self, _notif: NotificationWithId) -> Self::NotifyFuture {
            future::ok(()).boxed()
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
        let service_object =
            subject::ServiceObject::new(ServiceId::new(1), ObjectId::new(1)).unwrap();
        super::Subject::new(service_object, ActionId::new(1))
    }

    #[tokio::test]
    async fn test_session_pair_call() {
        let TestSessionPair {
            mut client,
            mut server,
        } = TestSessionPair::new().await;

        let subject = any_service_subject();
        let reply = client
            .call(Call::new(subject).with_value(&(12, -49)).unwrap())
            .await
            .unwrap();
        let value: String = reply.value().unwrap();
        assert_eq!(value, "-37");

        let reply = server
            .call(
                Call::new(subject)
                    .with_value(&vec![32, 2893, -123, 3287, 0, -38293])
                    .unwrap(),
            )
            .await
            .unwrap();
        let value: i32 = reply.value().unwrap();
        assert_eq!(value, -32204);
    }
}
