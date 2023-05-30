use crate::{
    capabilities, format,
    message::{self, Message, Subject},
};
use bytes::Bytes;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
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
    Capabilities {
        id: RequestId,
        capabilities: capabilities::Map,
    },
}

impl Request {
    pub fn id(&self) -> RequestId {
        match *self {
            Request::Call { id, .. } => id,
            Request::Post { id, .. } => id,
            Request::Event { id, .. } => id,
            Request::Cancel { id, .. } => id,
            Request::Capabilities { id, .. } => id,
        }
    }

    pub(crate) fn try_into_message(self) -> Result<Message, format::Error> {
        let message = match self {
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
            Request::Capabilities { id, capabilities } => {
                Message::capabilities(id.into(), &capabilities)?.build()
            }
        };
        Ok(message)
    }

    pub(crate) fn try_from_message(
        message: Message,
    ) -> Result<Result<Self, Message>, format::Error> {
        let request = match message.kind() {
            message::Kind::Call => Ok(Self::Call {
                id: message.id().into(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            message::Kind::Post => Ok(Self::Post {
                id: message.id().into(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            message::Kind::Event => Ok(Self::Event {
                id: message.id().into(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            message::Kind::Cancel => Ok(Self::Cancel {
                id: message.id().into(),
                subject: message.subject(),
                call_id: message.value()?,
            }),
            _ => Err(message),
        };
        Ok(request)
    }
}

impl TryFrom<Request> for Message {
    type Error = format::Error;

    fn try_from(request: Request) -> Result<Self, Self::Error> {
        request.try_into_message()
    }
}

impl TryFrom<Message> for Result<Request, Message> {
    type Error = format::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        Request::try_from_message(message)
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
    derive_more::From,
    derive_more::Into,
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

#[derive(
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Clone,
    Hash,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
pub struct Response(Option<CallResponse>);

impl Response {
    pub fn none() -> Self {
        Self(None)
    }

    pub fn reply(id: RequestId, subject: Subject, payload: Bytes) -> Self {
        Self(Some(CallResponse::reply(id, subject, payload)))
    }

    pub fn error(id: RequestId, subject: Subject, description: impl Into<String>) -> Self {
        Self(Some(CallResponse::error(id, subject, description)))
    }

    pub fn canceled(id: RequestId, subject: Subject) -> Self {
        Self(Some(CallResponse::canceled(id, subject)))
    }

    pub fn as_call_response(&self) -> Option<&CallResponse> {
        self.0.as_ref()
    }

    pub fn into_call_response(self) -> Option<CallResponse> {
        self.0
    }

    pub(crate) fn try_from_message(
        message: Message,
    ) -> Result<Result<Response, Message>, message::GetErrorDescriptionError> {
        CallResponse::try_from_message(message)
            .map(|response| response.map(|response| Self(Some(response))))
    }

    pub(crate) fn try_into_message(self) -> Result<Option<Message>, format::Error> {
        self.0.map(|r| r.try_into_message()).transpose()
    }
}

impl TryFrom<Message> for Result<Response, Message> {
    type Error = message::GetErrorDescriptionError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        Response::try_from_message(message)
    }
}

impl TryFrom<Response> for Option<Message> {
    type Error = format::Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        response.try_into_message()
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub struct CallResponse {
    id: RequestId,
    subject: Subject,
    kind: CallResponseKind,
}

impl CallResponse {
    pub fn reply(id: RequestId, subject: Subject, payload: Bytes) -> Self {
        Self {
            id,
            subject,
            kind: CallResponseKind::Reply(payload),
        }
    }

    pub fn error(id: RequestId, subject: Subject, description: impl Into<String>) -> Self {
        Self {
            id,
            subject,
            kind: CallResponseKind::Error(description.into()),
        }
    }

    pub fn canceled(id: RequestId, subject: Subject) -> Self {
        Self {
            id,
            subject,
            kind: CallResponseKind::Canceled,
        }
    }

    pub fn id(&self) -> RequestId {
        self.id
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn kind(&self) -> &CallResponseKind {
        &self.kind
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
    ) -> Result<Result<Self, Message>, message::GetErrorDescriptionError> {
        use message::Kind;
        let response = Self {
            id: message.id().into(),
            subject: message.subject(),
            kind: match message.kind() {
                Kind::Reply => CallResponseKind::Reply(message.into_payload()),
                Kind::Error => CallResponseKind::Error(message.error_description()?),
                Kind::Canceled => CallResponseKind::Canceled,
                _ => return Ok(Err(message)),
            },
        };
        Ok(Ok(response))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub enum CallResponseKind {
    Reply(Bytes),
    Error(String),
    Canceled,
}
