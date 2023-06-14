pub use super::service::Subject;
use super::Session;
use crate::{channel, format, request::Id};
use bytes::Bytes;
use futures::FutureExt;
use std::task::{Context, Poll};

#[derive(
    derive_new::new, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::From,
)]
pub enum Request {
    Call(Call),
    Post(Post),
    Event(Event),
    Cancel(Cancel),
}

impl Request {
    pub fn subject(&self) -> Subject {
        match self {
            Request::Call(call) => call.subject(),
            Request::Post(post) => post.subject(),
            Request::Event(event) => event.subject(),
            Request::Cancel(cancel) => cancel.subject(),
        }
    }

    pub(super) fn into_channel(self) -> channel::Request {
        match self {
            Request::Call(call) => call.into_channel().into(),
            Request::Post(post) => post.into_channel().into(),
            Request::Event(event) => event.into_channel().into(),
            Request::Cancel(cancel) => cancel.into_channel().into(),
        }
    }
}

#[derive(derive_new::new, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Call {
    subject: Subject,
    payload: Bytes,
}

impl Call {
    pub fn with_value<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self { subject, payload })
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    fn into_channel(self) -> channel::Call {
        channel::Call::new(self.subject.into(), self.payload)
    }
}

#[derive(derive_new::new, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Post {
    subject: Subject,
    payload: Bytes,
}

impl Post {
    pub fn with_value<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self { subject, payload })
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    fn into_channel(self) -> channel::Post {
        channel::Post::new(self.subject.into(), self.payload)
    }
}

#[derive(derive_new::new, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Event {
    subject: Subject,
    payload: Bytes,
}

impl Event {
    pub fn with_value<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self { subject, payload })
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    fn into_channel(self) -> channel::Event {
        channel::Event::new(self.subject.into(), self.payload)
    }
}

#[derive(derive_new::new, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Cancel {
    subject: Subject,
    call_id: Id,
}

impl Cancel {
    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn call_id(&self) -> Id {
        self.call_id
    }

    pub(super) fn into_channel(self) -> channel::Cancel {
        channel::Cancel::new(self.subject.into(), self.call_id)
    }
}

macro_rules! impl_service {
    ($(($req:ty, $rep:ty, $fut:ty)),+) => {
        $(
            impl tower::Service<$req> for Session {
                type Response = $rep;
                type Error = Error;
                type Future = $fut;

                fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                    self.poll_channel_ready(cx).map_err(Error)
                }

                fn call(&mut self, request: $req) -> Self::Future {
                    let request = request.into_channel();
                    self.channel.call(request).into()
                }
            }
        )+
    };
}

impl_service! {
    (Request, Option<Bytes>, Future),
    (Call, Bytes, CallResponseFuture),
    (Post, (), NoResponseFuture),
    (Event, (), NoResponseFuture),
    (Cancel, (), NoResponseFuture)
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(#[from] channel::Error);

#[derive(Debug, derive_more::From)]
#[must_use = "futures do nothing until polled"]
pub struct Future(channel::ResponseFuture);

impl Future {
    pub fn request_id(&self) -> Id {
        self.0.request_id()
    }
}

impl std::future::Future for Future {
    type Output = Result<Option<Bytes>, Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx).map_err(Into::into)
    }
}

#[derive(Debug, derive_more::From)]
#[must_use = "futures do nothing until polled"]
pub struct CallResponseFuture(channel::CallResponseFuture);

impl std::future::Future for CallResponseFuture {
    type Output = Result<Bytes, Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx).map_err(Into::into)
    }
}

impl CallResponseFuture {
    pub fn request_id(&self) -> Id {
        self.0.request_id()
    }
}

#[derive(Debug, derive_more::From)]
#[must_use = "futures do nothing until polled"]
pub struct NoResponseFuture(channel::NoResponseFuture);

impl std::future::Future for NoResponseFuture {
    type Output = Result<(), Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx).map_err(Into::into)
    }
}

impl NoResponseFuture {
    pub fn request_id(&self) -> Id {
        self.0.request_id()
    }
}
