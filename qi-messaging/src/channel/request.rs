use crate::{
    capabilities, format,
    message::Subject,
    request::{self, RequestId},
};
use bytes::Bytes;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
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

    pub fn into_messaging_request(self, id: RequestId) -> request::Request {
        match self {
            Self::Call { subject, payload } => request::Request::Call {
                id,
                subject,
                payload,
            },
            Self::Post { subject, payload } => request::Request::Post {
                id,
                subject,
                payload,
            },
            Self::Event { subject, payload } => request::Request::Event {
                id,
                subject,
                payload,
            },
            Self::Cancel { subject, call_id } => request::Request::Cancel {
                id,
                subject,
                call_id,
            },
            Self::Capabilities(capabilities) => request::Request::Capabilities { id, capabilities },
        }
    }
}
