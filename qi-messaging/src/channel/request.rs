pub use crate::request::Response;
use crate::{
    capabilities, format,
    message::{self, Subject},
    request::{Request as MessagingRequest, RequestId},
};
use bytes::Bytes;
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Clone, Debug)]
pub enum Request {
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
        call_id: RequestId,
    },
    Capabilities(capabilities::Map),
}

impl Request {
    pub fn call<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Call { subject, payload })
    }

    pub fn post<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Post { subject, payload })
    }

    pub fn event<T>(subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Event { subject, payload })
    }

    pub fn subject(&self) -> Subject {
        match self {
            Self::Call { subject, .. }
            | Self::Post { subject, .. }
            | Self::Event { subject, .. }
            | Self::Cancel { subject, .. } => *subject,
            Self::Capabilities(_) => message::CAPABILITIES_SUBJECT,
        }
    }

    pub fn into_messaging_request(self, id: RequestId) -> MessagingRequest {
        match self {
            Self::Call { subject, payload } => MessagingRequest::Call {
                id,
                subject,
                payload,
            },
            Self::Post { subject, payload } => MessagingRequest::Post {
                id,
                subject,
                payload,
            },
            Self::Event { subject, payload } => MessagingRequest::Event {
                id,
                subject,
                payload,
            },
            Self::Cancel { subject, call_id } => MessagingRequest::Cancel {
                id,
                subject,
                call_id,
            },
            Self::Capabilities(capabilities) => MessagingRequest::Capabilities { id, capabilities },
        }
    }
}

impl From<MessagingRequest> for Request {
    fn from(request: MessagingRequest) -> Self {
        match request {
            MessagingRequest::Call {
                subject, payload, ..
            } => Self::Call { subject, payload },
            MessagingRequest::Post {
                subject, payload, ..
            } => Self::Post { subject, payload },
            MessagingRequest::Event {
                subject, payload, ..
            } => Self::Event { subject, payload },
            MessagingRequest::Cancel {
                subject, call_id, ..
            } => Self::Cancel { subject, call_id },
            MessagingRequest::Capabilities { capabilities, .. } => Self::Capabilities(capabilities),
        }
    }
}

pin_project! {
    #[derive(derive_new::new, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub struct ResponseFuture<F> {
        request_id: RequestId,
        #[pin]
        inner: F,
    }
}

impl<F> ResponseFuture<F> {
    pub fn request_id(&self) -> RequestId {
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
