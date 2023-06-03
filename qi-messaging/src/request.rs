use super::message::{self, Message, Subject};
pub(crate) use crate::message::Id;
use crate::{capabilities::Map as Capabilities, format};
use bytes::Bytes;

#[derive(Clone, Debug)]
pub(crate) enum Request {
    Call {
        id: Id,
        subject: Subject,
        payload: Bytes,
    },
    Post {
        id: Id,
        subject: Subject,
        payload: Bytes,
    },
    Event {
        id: Id,
        subject: Subject,
        payload: Bytes,
    },
    Cancel {
        id: Id,
        subject: Subject,
        call_id: Id,
    },
    Capabilities {
        id: Id,
        subject: Subject,
        capabilities: Capabilities,
    },
}

impl Request {
    pub(crate) fn call<T>(id: Id, subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Call {
            id,
            subject,
            payload,
        })
    }

    pub(crate) fn post<T>(id: Id, subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Post {
            id,
            subject,
            payload,
        })
    }

    pub(crate) fn event<T>(id: Id, subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Event {
            id,
            subject,
            payload,
        })
    }

    pub(crate) fn id(&self) -> Id {
        use Request::*;
        match *self {
            Call { id, .. }
            | Post { id, .. }
            | Event { id, .. }
            | Cancel { id, .. }
            | Capabilities { id, .. } => id,
        }
    }

    pub(crate) fn subject(&self) -> Subject {
        use Request::*;
        match *self {
            Call { subject, .. }
            | Post { subject, .. }
            | Event { subject, .. }
            | Cancel { subject, .. }
            | Capabilities { subject, .. } => subject,
        }
    }

    pub(crate) fn try_into_message(self) -> Result<Message, format::Error> {
        use Request::*;
        let message = match self {
            Call {
                id,
                subject,
                payload,
            } => Message::call(id, subject).set_payload(payload).build(),
            Post {
                id,
                subject,
                payload,
            } => Message::post(id, subject).set_payload(payload).build(),
            Event {
                id,
                subject,
                payload,
            } => Message::event(id, subject).set_payload(payload).build(),
            Cancel {
                id,
                subject,
                call_id,
            } => Message::cancel(id, subject, call_id).build(),
            Capabilities {
                id,
                subject,
                capabilities,
            } => Message::capabilities(id, subject, &capabilities)?.build(),
        };
        Ok(message)
    }

    pub(crate) fn try_from_message(
        message: Message,
    ) -> Result<Result<Self, Message>, format::Error> {
        use message::Kind::*;
        let request = match message.kind() {
            Call => Ok(Self::Call {
                id: message.id(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            Post => Ok(Self::Post {
                id: message.id(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            Event => Ok(Self::Event {
                id: message.id(),
                subject: message.subject(),
                payload: message.into_payload(),
            }),
            Cancel => Ok(Self::Cancel {
                id: message.id(),
                subject: message.subject(),
                call_id: message.value()?,
            }),
            _ => Err(message),
        };
        Ok(request)
    }
}

#[derive(
    derive_new::new, Default, Debug, derive_more::From, derive_more::Into, derive_more::AsRef,
)]
pub struct Response(pub(crate) Option<CallResult>);

impl Response {
    pub fn none() -> Self {
        Self(None)
    }

    pub fn reply<T>(value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        Ok(Self(Some(CallResult::reply(value)?)))
    }

    pub fn error<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Sync + Send>>,
    {
        Self(Some(CallResult::Error(error.into())))
    }

    pub fn canceled() -> Self {
        Self(Some(CallResult::Canceled))
    }

    pub(crate) fn as_call_result(&self) -> Option<&CallResult> {
        self.0.as_ref()
    }

    pub(crate) fn into_call_result(self) -> Option<CallResult> {
        self.0
    }

    pub fn into_result<T>(self) -> Result<Option<T>, CallError>
    where
        T: serde::de::DeserializeOwned,
    {
        match self.0 {
            Some(call_result) => {
                let result = call_result.into_result()?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    pub(crate) fn try_from_message(
        message: message::Message,
    ) -> Result<Result<(Id, Response), message::Message>, message::GetErrorDescriptionError> {
        CallResult::try_from_message(message)
            .map(|response| response.map(|(id, response)| (id, Self(Some(response)))))
    }

    pub(crate) fn try_into_message(
        self,
        id: Id,
        subject: message::Subject,
    ) -> Result<Option<message::Message>, format::Error> {
        self.0.map(|r| r.try_into_message(id, subject)).transpose()
    }
}

#[derive(Debug)]
pub enum CallResult {
    Reply(Bytes),
    Error(Box<dyn std::error::Error + Sync + Send>),
    Canceled,
}

impl CallResult {
    pub fn reply<T>(value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self::Reply(payload))
    }

    pub fn into_result<T>(self) -> Result<T, CallError>
    where
        T: serde::de::DeserializeOwned,
    {
        use CallResult::*;
        match self {
            Reply(payload) => {
                let value = format::from_bytes(&payload).map_err(CallError::ReplyPayloadFormat)?;
                Ok(value)
            }
            Error(description) => Err(CallError::Error(description)),
            Canceled => Err(CallError::Canceled),
        }
    }

    fn try_into_message(
        self,
        id: Id,
        subject: message::Subject,
    ) -> Result<message::Message, format::Error> {
        use CallResult::*;
        Ok(match self {
            Reply(payload) => message::Message::reply(id, subject)
                .set_payload(payload)
                .build(),
            Error(err) => message::Message::error(id, subject, &err.to_string())?.build(),
            Canceled => message::Message::canceled(id, subject).build(),
        })
    }

    fn try_from_message(
        message: message::Message,
    ) -> Result<Result<(Id, Self), message::Message>, message::GetErrorDescriptionError> {
        use message::Kind::*;
        let id = message.id();
        let response = match message.kind() {
            Reply => Self::Reply(message.into_payload()),
            Error => Self::Error(message.error_description()?.into()),
            Canceled => Self::Canceled,
            _ => return Ok(Err(message)),
        };
        Ok(Ok((id, response)))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("error deserializing the value from the reply payload")]
    ReplyPayloadFormat(#[from] format::Error),

    #[error(transparent)]
    Error(#[from] Box<dyn std::error::Error + Sync + Send>),

    #[error("the call request has been canceled")]
    Canceled,
}
