use crate::{format, message};
pub use message::Id as RequestId;
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub trait Service<C, N> {
    type CallReply;
    type Error;
    type CallFuture: Future<Output = CallResult<Self::CallReply, Self::Error>>;
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

pub trait GetSubject {
    type Subject;
    fn subject(&self) -> &Self::Subject;
}

pub trait ToRequestId {
    fn to_request_id(&self) -> RequestId;
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Request<C, N> {
    Call(C),
    Notification(N),
}

impl<C, N, S> GetSubject for Request<C, N>
where
    C: GetSubject<Subject = S>,
    N: GetSubject<Subject = S>,
{
    type Subject = S;
    fn subject(&self) -> &Self::Subject {
        match self {
            Self::Call(call) => call.subject(),
            Self::Notification(notif) => notif.subject(),
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
    subject: S,
    formatted_value: format::Value,
}

pub(crate) type CallWithId<S> = WithRequestId<Call<S>>;

impl<S> Call<S> {
    pub fn new(subject: S) -> Self {
        Self {
            subject,
            formatted_value: format::Value::new(),
        }
    }

    pub(crate) fn with_formatted_value(mut self, formatted_value: format::Value) -> Self {
        self.formatted_value = formatted_value;
        self
    }

    pub(crate) fn into_formatted_value(self) -> format::Value {
        self.formatted_value
    }

    pub fn with_value<T>(mut self, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        self.formatted_value = format::Value::from_serializable(value)?;
        Ok(self)
    }

    pub fn value<'de, T>(&'de self) -> Result<T, format::Error>
    where
        T: serde::Deserialize<'de>,
    {
        self.formatted_value.to_deserializable()
    }
}

impl<S> GetSubject for Call<S> {
    type Subject = S;

    fn subject(&self) -> &Self::Subject {
        &self.subject
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Post<S> {
    subject: S,
    formatted_value: format::Value,
}

impl<S> Post<S> {
    pub(crate) fn new(subject: S) -> Self {
        Self {
            subject,
            formatted_value: format::Value::new(),
        }
    }

    pub(crate) fn with_formatted_value(mut self, formatted_value: format::Value) -> Self {
        self.formatted_value = formatted_value;
        self
    }

    pub(crate) fn into_formatted_value(self) -> format::Value {
        self.formatted_value
    }
}

pub(crate) type PostWithId<S> = WithRequestId<Post<S>>;

impl<S> GetSubject for Post<S> {
    type Subject = S;

    fn subject(&self) -> &Self::Subject {
        &self.subject
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Event<S> {
    subject: S,
    formatted_value: format::Value,
}

impl<S> Event<S> {
    pub(crate) fn new(subject: S) -> Self {
        Self {
            subject,
            formatted_value: format::Value::new(),
        }
    }

    pub(crate) fn with_formatted_value(mut self, formatted_value: format::Value) -> Self {
        self.formatted_value = formatted_value;
        self
    }

    pub(crate) fn into_formatted_value(self) -> format::Value {
        self.formatted_value
    }
}

pub(crate) type EventWithId<S> = WithRequestId<Event<S>>;

impl<S> GetSubject for Event<S> {
    type Subject = S;

    fn subject(&self) -> &Self::Subject {
        &self.subject
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Cancel<S> {
    subject: S,
    call_id: RequestId,
}

impl<S> Cancel<S> {
    pub(crate) fn new(subject: S, call_id: RequestId) -> Self {
        Self { subject, call_id }
    }

    pub(crate) fn call_id(&self) -> RequestId {
        self.call_id
    }
}

pub(crate) type CancelWithId<S> = WithRequestId<Cancel<S>>;

impl<S> GetSubject for Cancel<S> {
    type Subject = S;

    fn subject(&self) -> &Self::Subject {
        &self.subject
    }
}

#[derive(
    derive_new::new,
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    derive_more::From,
)]
pub struct WithRequestId<T> {
    id: RequestId,
    inner: T,
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

impl<T> GetSubject for WithRequestId<T>
where
    T: GetSubject,
{
    type Subject = T::Subject;

    fn subject(&self) -> &Self::Subject {
        self.inner.subject()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallTermination<E> {
    #[error("the call request has been canceled")]
    Canceled,

    #[error(transparent)]
    Error(#[from] E),
}

impl<E> CallTermination<E> {
    pub fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }

    pub fn error(&self) -> Option<&E> {
        match self {
            Self::Canceled => None,
            Self::Error(err) => Some(err),
        }
    }

    pub fn map_err<F, ToE>(self, f: F) -> CallTermination<ToE>
    where
        F: FnOnce(E) -> ToE,
    {
        match self {
            Self::Canceled => CallTermination::Canceled,
            Self::Error(err) => CallTermination::Error(f(err)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, derive_more::Into)]
pub struct Reply {
    formatted_value: format::Value,
}

impl Reply {
    pub(crate) fn new(formatted_value: format::Value) -> Self {
        Self { formatted_value }
    }

    pub fn with_value<T>(value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        Ok(Self {
            formatted_value: format::Value::from_serializable(value)?,
        })
    }

    pub fn value<'de, T>(&'de self) -> Result<T, format::Error>
    where
        T: serde::Deserialize<'de>,
    {
        self.formatted_value.to_deserializable()
    }
}

pub type CallResult<T, E> = Result<T, CallTermination<E>>;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, thiserror::Error, derive_more::From,
)]
#[error("the call request ended with an error: {0}")]
pub struct Error(pub(crate) String);

impl Error {
    pub fn reason(&self) -> &str {
        &self.0
    }
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

impl<Call, Notif, T, E> Future for RequestFuture<Call, Notif>
where
    Call: Future<Output = CallResult<T, E>>,
    Notif: Future<Output = Result<(), E>>,
{
    type Output = Result<Option<T>, CallTermination<E>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            RequestFutureProj::Call { inner } => inner.poll(cx)?.map(|reply| Ok(Some(reply))),
            RequestFutureProj::Notification { inner } => inner.poll(cx).map(|res| match res {
                Ok(()) => Ok(None),
                Err(err) => Err(CallTermination::Error(err)),
            }),
        }
    }
}
