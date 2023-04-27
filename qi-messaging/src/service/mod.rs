pub(crate) mod client;
pub(crate) mod server;

use crate::{
    format,
    message::{self, Message, Subject},
};
use bytes::Bytes;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Request {
    Call {
        id: RequestId,
        subject: Subject,
        payload: Bytes,
    },
    Post {
        id: RequestId,
        subject: Subject,
        payload: Bytes,
    },
    Event {
        id: RequestId,
        subject: Subject,
        payload: Bytes,
    },
    Cancel {
        id: RequestId,
        subject: Subject,
        call_id: RequestId,
    },
}

impl Request {
    fn id(&self) -> RequestId {
        match *self {
            Request::Call { id, .. } => id,
            Request::Post { id, .. } => id,
            Request::Event { id, .. } => id,
            Request::Cancel { id, .. } => id,
        }
    }

    fn into_message(self) -> Message {
        match self {
            Self::Call {
                id,
                subject,
                payload,
            } => Message::call(id.into(), subject)
                .set_payload(payload)
                .build(),
            Request::Post {
                id,
                subject,
                payload,
            } => Message::post(id.into(), subject)
                .set_payload(payload)
                .build(),
            Request::Event {
                id,
                subject,
                payload,
            } => Message::event(id.into(), subject)
                .set_payload(payload)
                .build(),
            Request::Cancel {
                id,
                subject,
                call_id,
            } => Message::cancel(id.into(), subject, call_id.into()).build(),
        }
    }

    fn try_from_message(message: Message) -> Result<Option<Self>, format::Error> {
        let request = match message.kind() {
            message::Kind::Call => Some(Self::Call {
                id: message.id().into(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            message::Kind::Post => Some(Self::Post {
                id: message.id().into(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            message::Kind::Event => Some(Self::Event {
                id: message.id().into(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            message::Kind::Cancel => Some(Self::Cancel {
                id: message.id().into(),
                subject: message.subject(),
                call_id: message.value()?,
            }),
            _ => None,
        };
        Ok(request)
    }
}

#[derive(
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(from = "message::Id", into = "message::Id")]
pub struct RequestId(u32);

#[doc(hidden)]
impl From<message::Id> for RequestId {
    fn from(value: message::Id) -> Self {
        Self(value.0)
    }
}

#[doc(hidden)]
impl From<RequestId> for message::Id {
    fn from(value: RequestId) -> Self {
        Self(value.0)
    }
}

type Response = Option<CallResponse>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub struct CallResponse {
    id: RequestId,
    subject: Subject,
    kind: CallResponseKind,
}

impl CallResponse {
    fn reply(id: RequestId, subject: Subject, payload: Bytes) -> Self {
        Self {
            id,
            subject,
            kind: CallResponseKind::Reply(payload),
        }
    }

    fn error(id: RequestId, subject: Subject, description: impl Into<String>) -> Self {
        Self {
            id,
            subject,
            kind: CallResponseKind::Error(description.into()),
        }
    }

    fn canceled(id: RequestId, subject: Subject) -> Self {
        Self {
            id,
            subject,
            kind: CallResponseKind::Canceled,
        }
    }

    fn try_into_message(self) -> Result<Message, format::Error> {
        Ok(match self.kind {
            CallResponseKind::Reply(payload) => Message::reply(self.id.into(), self.subject)
                .set_payload(payload)
                .build(),
            CallResponseKind::Error(descr) => {
                Message::error(self.id.into(), self.subject, &descr)?.build()
            }
            CallResponseKind::Canceled => Message::canceled(self.id.into(), self.subject).build(),
        })
    }

    fn try_from_message(
        message: Message,
    ) -> Result<Option<Self>, message::GetErrorDescriptionError> {
        use message::Kind;
        let response = Self {
            id: message.id().into(),
            subject: message.subject(),
            kind: match message.kind() {
                Kind::Reply => CallResponseKind::Reply(message.into_payload()),
                Kind::Error => CallResponseKind::Error(message.error_description()?),
                Kind::Canceled => CallResponseKind::Canceled,
                _ => return Ok(None),
            },
        };
        Ok(Some(response))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub enum CallResponseKind {
    Reply(Bytes),
    Error(String),
    Canceled,
}

fn try_response_from_message(
    message: Message,
) -> Result<Response, message::GetErrorDescriptionError> {
    CallResponse::try_from_message(message)
}
