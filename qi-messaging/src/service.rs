use crate::{
    format,
    message::{self, Message},
    Bytes,
};
pub use message::Id as RequestId;
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub trait Service<C, N> {
    type Error;
    type CallFuture: Future<Output = Result<Bytes, Self::Error>>;
    type NotifyFuture: Future<Output = Result<(), Self::Error>>;

    fn call(&mut self, call: C) -> Self::CallFuture;
    fn notify(&mut self, notif: N) -> Self::NotifyFuture;

    fn request(
        &mut self,
        request: Request<C, N>,
    ) -> RequestFuture<Self::CallFuture, Self::NotifyFuture> {
        match request {
            Request::Call(call) => RequestFuture::Call {
                inner: self.call(call),
            },
            Request::Notification(notif) => RequestFuture::Notification {
                inner: self.notify(notif),
            },
        }
    }
}
pub trait ToSubject {
    type Subject;
    fn to_subject(&self) -> Self::Subject;
}

pub trait ToRequestId {
    fn to_request_id(&self) -> RequestId;
}

pub(crate) trait TryIntoMessageWithId: Sized {
    fn try_into_message(self, id: message::Id) -> Result<Message, format::Error>;
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Request<C, N> {
    Call(C),
    Notification(N),
}

impl<C, N, S> ToSubject for Request<C, N>
where
    C: ToSubject<Subject = S>,
    N: ToSubject<Subject = S>,
{
    type Subject = S;
    fn to_subject(&self) -> Self::Subject {
        match self {
            Self::Call(call) => call.to_subject(),
            Self::Notification(notif) => notif.to_subject(),
        }
    }
}

impl<C, N> ToRequestId for Request<C, N>
where
    C: ToRequestId,
    N: ToRequestId,
{
    fn to_request_id(&self) -> RequestId {
        match self {
            Self::Call(call) => call.to_request_id(),
            Self::Notification(notif) => notif.to_request_id(),
        }
    }
}

impl<C, N> TryIntoMessageWithId for Request<C, N>
where
    C: TryIntoMessageWithId,
    N: TryIntoMessageWithId,
{
    fn try_into_message(self, id: message::Id) -> Result<Message, format::Error> {
        match self {
            Request::Call(call) => call.try_into_message(id),
            Request::Notification(notif) => notif.try_into_message(id),
        }
    }
}

impl<C, N> WithRequestId<Request<C, N>> {
    pub fn transpose_id(self) -> Request<WithRequestId<C>, WithRequestId<N>> {
        let WithRequestId { id, inner } = self;
        match inner {
            Request::Call(inner) => Request::Call(WithRequestId { id, inner }),
            Request::Notification(inner) => Request::Notification(WithRequestId { id, inner }),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Call<S> {
    pub(crate) subject: S,
    pub(crate) payload: Bytes,
}

impl<S> Call<S> {
    pub fn with_content<T>(subject: S, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        Ok(Self {
            subject,
            payload: format::to_bytes(value)?,
        })
    }

    pub fn content<'de, T>(&'de self) -> Result<T, format::Error>
    where
        T: serde::Deserialize<'de>,
    {
        format::from_bytes(&self.payload)
    }
}

impl<S> ToSubject for Call<S>
where
    S: Copy,
{
    type Subject = S;

    fn to_subject(&self) -> Self::Subject {
        self.subject
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Post<S> {
    pub(crate) subject: S,
    pub(crate) payload: Bytes,
}

impl<S> ToSubject for Post<S>
where
    S: Copy,
{
    type Subject = S;

    fn to_subject(&self) -> Self::Subject {
        self.subject
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Event<S> {
    pub(crate) subject: S,
    pub(crate) payload: Bytes,
}

impl<S> ToSubject for Event<S>
where
    S: Copy,
{
    type Subject = S;

    fn to_subject(&self) -> Self::Subject {
        self.subject
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Cancel<S> {
    pub(crate) subject: S,
    pub(crate) call_id: RequestId,
}

impl<S> ToSubject for Cancel<S>
where
    S: Copy,
{
    type Subject = S;

    fn to_subject(&self) -> Self::Subject {
        self.subject
    }
}

#[derive(derive_new::new, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct WithRequestId<T> {
    pub(crate) id: RequestId,
    pub(crate) inner: T,
}

impl<T> WithRequestId<T> {
    pub fn id(&self) -> RequestId {
        self.id
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> ToRequestId for WithRequestId<T> {
    fn to_request_id(&self) -> RequestId {
        self.id
    }
}

impl<T> ToSubject for WithRequestId<T>
where
    T: ToSubject,
{
    type Subject = T::Subject;

    fn to_subject(&self) -> Self::Subject {
        self.inner.to_subject()
    }
}

impl<T> TryFrom<WithRequestId<T>> for Message
where
    T: TryIntoMessageWithId,
{
    type Error = format::Error;

    fn try_from(value: WithRequestId<T>) -> Result<Self, Self::Error> {
        value.inner.try_into_message(value.id)
    }
}

pub(crate) type CallWithId<S> = WithRequestId<Call<S>>;
pub(crate) type PostWithId<S> = WithRequestId<Post<S>>;
pub(crate) type EventWithId<S> = WithRequestId<Event<S>>;
pub(crate) type CancelWithId<S> = WithRequestId<Cancel<S>>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CallTermination {
    #[error("the call request has been canceled")]
    Canceled,

    #[error("the call request ended with an error: {0}")]
    Error(String),
}

pub trait IsErrorCanceledTermination {
    fn is_canceled(&self) -> bool;
}

impl IsErrorCanceledTermination for CallTermination {
    fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }
}

impl<'t, T> IsErrorCanceledTermination for &'t T
where
    T: IsErrorCanceledTermination + ?Sized,
{
    fn is_canceled(&self) -> bool {
        <T as IsErrorCanceledTermination>::is_canceled(*self)
    }
}

impl<T> IsErrorCanceledTermination for Box<T>
where
    T: IsErrorCanceledTermination,
{
    fn is_canceled(&self) -> bool {
        <T as IsErrorCanceledTermination>::is_canceled(self.as_ref())
    }
}

impl<T> IsErrorCanceledTermination for std::sync::Arc<T>
where
    T: IsErrorCanceledTermination,
{
    fn is_canceled(&self) -> bool {
        <T as IsErrorCanceledTermination>::is_canceled(self.as_ref())
    }
}

macro_rules! impl_is_never_canceled_termination {
    ($($err:ty),+) => {
        $(impl IsErrorCanceledTermination for $err {
            fn is_canceled(&self) -> bool {
                false
            }
        })+
    };
}

impl_is_never_canceled_termination! {
    std::io::Error,
    std::convert::Infallible,
    std::fmt::Error,
    std::num::ParseIntError,
    std::num::ParseFloatError,
    std::num::TryFromIntError,
    std::str::ParseBoolError,
    std::str::Utf8Error,
    std::string::FromUtf8Error,
    Box<dyn std::error::Error>,
    Box<dyn std::error::Error + Send + Sync>
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing until polled"]
    #[project = RequestFutureProj]
    pub enum RequestFuture<Call, Notif> {
        Call {
            #[pin]
            inner: Call
        },
        Notification {
            #[pin]
            inner: Notif
        },
    }
}

impl<Call, Notif, E> Future for RequestFuture<Call, Notif>
where
    Call: Future<Output = Result<Bytes, E>>,
    Notif: Future<Output = Result<(), E>>,
{
    type Output = Result<Option<Bytes>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            RequestFutureProj::Call { inner } => inner.poll(cx)?.map(|reply| Ok(Some(reply))),
            RequestFutureProj::Notification { inner } => inner.poll(cx)?.map(|()| Ok(None)),
        }
    }
}
