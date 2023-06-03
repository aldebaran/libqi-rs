use crate::{
    capabilities::Map as Capabilities,
    format,
    message::Subject,
    request::{Id, Request as MessagingRequest},
};
use bytes::Bytes;
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Clone, Debug)]
pub(crate) enum Request {
    Call {
        subject: Subject,
        payload: Bytes,
    },
    Post {
        subject: Subject,
        payload: Bytes,
    },
    Event {
        subject: Subject,
        payload: Bytes,
    },
    Cancel {
        subject: Subject,
        call_id: Id,
    },
    Capabilities {
        subject: Subject,
        capabilities: Capabilities,
    },
}

impl Request {
    pub(crate) fn call<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Call { subject, payload })
    }

    pub(crate) fn post<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Post { subject, payload })
    }

    pub(crate) fn event<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Event { subject, payload })
    }

    pub(crate) fn subject(&self) -> Subject {
        use Request::*;
        match self {
            Call { subject, .. }
            | Post { subject, .. }
            | Event { subject, .. }
            | Cancel { subject, .. }
            | Capabilities { subject, .. } => *subject,
        }
    }

    pub(crate) fn into_messaging_request(self, id: Id) -> MessagingRequest {
        use Request::*;
        match self {
            Call { subject, payload } => MessagingRequest::Call {
                id,
                subject,
                payload,
            },
            Post { subject, payload } => MessagingRequest::Post {
                id,
                subject,
                payload,
            },
            Event { subject, payload } => MessagingRequest::Event {
                id,
                subject,
                payload,
            },
            Cancel { subject, call_id } => MessagingRequest::Cancel {
                id,
                subject,
                call_id,
            },
            Capabilities {
                subject,
                capabilities,
            } => MessagingRequest::Capabilities {
                id,
                subject,
                capabilities,
            },
        }
    }
}

impl From<MessagingRequest> for Request {
    fn from(request: MessagingRequest) -> Self {
        use MessagingRequest::*;
        match request {
            Call {
                subject, payload, ..
            } => Self::Call { subject, payload },
            Post {
                subject, payload, ..
            } => Self::Post { subject, payload },
            Event {
                subject, payload, ..
            } => Self::Event { subject, payload },
            Cancel {
                subject, call_id, ..
            } => Self::Cancel { subject, call_id },
            Capabilities {
                subject,
                capabilities,
                ..
            } => Self::Capabilities {
                subject,
                capabilities,
            },
        }
    }
}

pin_project! {
    #[derive(derive_new::new, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
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
