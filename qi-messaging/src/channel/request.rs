use crate::{capabilities, format, message::Subject, request::Id};
use bytes::Bytes;
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug, derive_more::From)]
pub(crate) enum Request {
    Call(Call),
    Post(Post),
    Event(Event),
    Cancel(Cancel),
    Capabilities(Capabilities),
}

impl Request {
    pub(crate) fn subject(&self) -> Subject {
        match self {
            Request::Call(Call { subject, .. })
            | Request::Post(Post { subject, .. })
            | Request::Event(Event { subject, .. })
            | Request::Cancel(Cancel { subject, .. })
            | Request::Capabilities(Capabilities { subject, .. }) => *subject,
        }
    }

    pub(crate) fn into_messaging(self, id: Id) -> crate::request::Request {
        match self {
            Request::Call(call) => call.into_messaging(id).into(),
            Request::Post(post) => post.into_messaging(id).into(),
            Request::Event(event) => event.into_messaging(id).into(),
            Request::Cancel(cancel) => cancel.into_messaging(id).into(),
            Request::Capabilities(capabilities) => capabilities.into_messaging(id).into(),
        }
    }
}

impl From<crate::request::Request> for Request {
    fn from(request: crate::request::Request) -> Self {
        use crate::request::Request;
        match request {
            Request::Call(crate::request::Call {
                subject, payload, ..
            }) => Call { subject, payload }.into(),
            Request::Post(crate::request::Post {
                subject, payload, ..
            }) => Post { subject, payload }.into(),
            Request::Event(crate::request::Event {
                subject, payload, ..
            }) => Event { subject, payload }.into(),
            Request::Cancel(crate::request::Cancel {
                subject, call_id, ..
            }) => Cancel { subject, call_id }.into(),
            Request::Capabilities(crate::request::Capabilities {
                subject,
                capabilities,
                ..
            }) => Capabilities {
                subject,
                capabilities,
            }
            .into(),
        }
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Call {
    subject: Subject,
    payload: Bytes,
}

impl Call {
    pub(crate) fn with_value<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self { subject, payload })
    }

    pub(crate) fn into_messaging(self, id: Id) -> crate::request::Call {
        crate::request::Call {
            id,
            subject: self.subject,
            payload: self.payload,
        }
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Post {
    subject: Subject,
    payload: Bytes,
}

impl Post {
    pub(crate) fn with_value<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self { subject, payload })
    }

    pub(crate) fn into_messaging(self, id: Id) -> crate::request::Post {
        crate::request::Post {
            id,
            subject: self.subject,
            payload: self.payload,
        }
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Event {
    subject: Subject,
    payload: Bytes,
}

impl Event {
    pub(crate) fn with_value<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self { subject, payload })
    }

    pub(crate) fn into_messaging(self, id: Id) -> crate::request::Event {
        crate::request::Event {
            id,
            subject: self.subject,
            payload: self.payload,
        }
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Cancel {
    subject: Subject,
    call_id: Id,
}

impl Cancel {
    pub(crate) fn into_messaging(self, id: Id) -> crate::request::Cancel {
        crate::request::Cancel {
            id,
            subject: self.subject,
            call_id: self.call_id,
        }
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Capabilities {
    subject: Subject,
    capabilities: capabilities::Map,
}

impl Capabilities {
    pub(crate) fn into_messaging(self, id: Id) -> crate::request::Capabilities {
        crate::request::Capabilities {
            id,
            subject: self.subject,
            capabilities: self.capabilities,
        }
    }
}

pin_project! {
    #[derive(derive_new::new, Debug)]
    #[must_use = "futures do nothing until polled"]
    pub(crate) struct Future<F> {
        request_id: Id,
        #[pin]
        inner: F,
    }
}

impl<F> Future<F> {
    pub(crate) fn request_id(&self) -> Id {
        self.request_id
    }
}

impl<F> std::future::Future for Future<F>
where
    F: std::future::Future,
{
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}
