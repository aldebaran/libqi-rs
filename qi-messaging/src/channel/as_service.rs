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
    pub(crate) fn into_messaging(self, id: Id) -> crate::Request {
        match self {
            Request::Call(call) => call.into_messaging(id).into(),
            Request::Post(post) => post.into_messaging(id).into(),
            Request::Event(event) => event.into_messaging(id).into(),
            Request::Cancel(cancel) => cancel.into_messaging(id).into(),
            Request::Capabilities(capabilities) => capabilities.into_messaging(id).into(),
        }
    }
}

impl From<crate::Request> for Request {
    fn from(request: crate::Request) -> Self {
        use crate::Request;
        match request {
            Request::Call(crate::Call {
                subject, payload, ..
            }) => Call { subject, payload }.into(),
            Request::Post(crate::Post {
                subject, payload, ..
            }) => Post { subject, payload }.into(),
            Request::Event(crate::Event {
                subject, payload, ..
            }) => Event { subject, payload }.into(),
            Request::Cancel(crate::Cancel {
                subject, call_id, ..
            }) => Cancel { subject, call_id }.into(),
            Request::Capabilities(crate::Capabilities {
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

    pub(crate) fn into_messaging(self, id: Id) -> crate::Call {
        crate::Call {
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
    pub(crate) fn into_messaging(self, id: Id) -> crate::Post {
        crate::Post {
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
    pub(crate) fn into_messaging(self, id: Id) -> crate::Event {
        crate::Event {
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
    pub(crate) fn into_messaging(self, id: Id) -> crate::Cancel {
        crate::Cancel {
            id,
            subject: self.subject,
            call_id: self.call_id,
        }
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Capabilities {
    subject: Subject,
    capabilities: capabilities::CapabilitiesMap,
}

impl Capabilities {
    pub(crate) fn into_messaging(self, id: Id) -> crate::Capabilities {
        crate::Capabilities {
            id,
            subject: self.subject,
            capabilities: self.capabilities,
        }
    }
}

pin_project! {
    #[derive(derive_new::new, Debug)]
    #[must_use = "futures do nothing until polled"]
    pub(crate) struct ResponseFuture<F> {
        request_id: Id,
        #[pin]
        inner: F,
    }
}

impl<F> ResponseFuture<F> {
    pub(crate) fn request_id(&self) -> Id {
        self.request_id
    }
}

impl<F> std::future::Future for ResponseFuture<F>
where
    F: std::future::Future,
{
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}
