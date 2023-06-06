use super::message::{self, Message, Subject};
pub(crate) use crate::message::Id;
use crate::{capabilities, format};
use bytes::Bytes;
use sealed::sealed;

#[derive(Debug, derive_more::From)]
pub(crate) enum Request {
    Call(Call),
    Post(Post),
    Event(Event),
    Cancel(Cancel),
    Capabilities(Capabilities),
}

impl Request {
    pub(crate) fn id(&self) -> Id {
        match *self {
            Request::Call(Call { id, .. })
            | Request::Post(Post { id, .. })
            | Request::Event(Event { id, .. })
            | Request::Cancel(Cancel { id, .. })
            | Request::Capabilities(Capabilities { id, .. }) => id,
        }
    }

    pub(crate) fn subject(&self) -> Subject {
        match *self {
            Request::Call(Call { subject, .. })
            | Request::Post(Post { subject, .. })
            | Request::Event(Event { subject, .. })
            | Request::Cancel(Cancel { subject, .. })
            | Request::Capabilities(Capabilities { subject, .. }) => subject,
        }
    }

    pub(crate) fn try_into_message(self) -> Result<Message, format::Error> {
        let message = match self {
            Request::Call(call) => call.into_message(),
            Request::Post(Post {
                id,
                subject,
                payload,
            }) => Message::post(id, subject).set_payload(payload).build(),
            Request::Event(Event {
                id,
                subject,
                payload,
            }) => Message::event(id, subject).set_payload(payload).build(),
            Request::Cancel(Cancel {
                id,
                subject,
                call_id,
            }) => Message::cancel(id, subject, call_id).build(),
            Request::Capabilities(Capabilities {
                id,
                subject,
                capabilities,
            }) => Message::capabilities(id, subject, &capabilities)?.build(),
        };
        Ok(message)
    }

    pub(crate) fn try_from_message(
        message: Message,
    ) -> Result<Result<Self, Message>, format::Error> {
        let request = match message.kind() {
            message::Kind::Call => Ok(Self::Call(Call {
                id: message.id(),
                subject: message.subject(),
                payload: message.into_payload(),
            })),
            message::Kind::Post => Ok(Self::Post(Post {
                id: message.id(),
                subject: message.subject(),
                payload: message.into_payload(),
            })),
            message::Kind::Event => Ok(Self::Event(Event {
                id: message.id(),
                subject: message.subject(),
                payload: message.into_payload(),
            })),
            message::Kind::Cancel => Ok(Self::Cancel(Cancel {
                id: message.id(),
                subject: message.subject(),
                call_id: message.value()?,
            })),
            _ => Err(message),
        };
        Ok(request)
    }
}

impl TryFrom<Request> for Message {
    type Error = format::Error;

    fn try_from(req: Request) -> Result<Self, Self::Error> {
        req.try_into_message()
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Call {
    pub(crate) id: Id,
    pub(crate) subject: Subject,
    pub(crate) payload: Bytes,
}

impl Call {
    pub(crate) fn with_value<T>(id: Id, subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self {
            id,
            subject,
            payload,
        })
    }

    pub(crate) fn value<'de, T>(&'de self) -> Result<T, format::Error>
    where
        T: serde::Deserialize<'de>,
    {
        format::from_bytes(&self.payload)
    }

    pub(crate) fn into_message(self) -> Message {
        Message::call(self.id, self.subject)
            .set_payload(self.payload)
            .build()
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Post {
    pub(crate) id: Id,
    pub(crate) subject: Subject,
    pub(crate) payload: Bytes,
}

impl Post {
    pub(crate) fn with_value<T>(id: Id, subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self {
            id,
            subject,
            payload,
        })
    }

    pub(crate) fn into_message(self) -> Message {
        Message::post(self.id, self.subject)
            .set_payload(self.payload)
            .build()
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Event {
    pub(crate) id: Id,
    pub(crate) subject: Subject,
    pub(crate) payload: Bytes,
}

impl Event {
    pub(crate) fn with_value<T>(id: Id, subject: Subject, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        let payload = format::to_bytes(value)?;
        Ok(Self {
            id,
            subject,
            payload,
        })
    }

    pub(crate) fn into_message(self) -> Message {
        Message::event(self.id, self.subject)
            .set_payload(self.payload)
            .build()
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Cancel {
    pub(crate) id: Id,
    pub(crate) subject: Subject,
    pub(crate) call_id: Id,
}

impl Cancel {
    pub(crate) fn into_message(self) -> Message {
        Message::cancel(self.id, self.subject, self.call_id).build()
    }
}

#[derive(derive_new::new, Debug, derive_more::Into)]
pub(crate) struct Capabilities {
    pub(crate) id: Id,
    pub(crate) subject: Subject,
    #[into]
    pub(crate) capabilities: capabilities::Map,
}

impl Capabilities {
    pub(crate) fn try_into_message(self) -> Result<Message, format::Error> {
        Ok(Message::capabilities(self.id, self.subject, &self.capabilities)?.build())
    }
}

pub trait IsCanceled {
    fn is_canceled(&self) -> bool;
}

#[sealed]
pub(crate) trait TryIntoFailureMessage {
    fn try_into_failure_message(self, id: Id, subject: Subject) -> Result<Message, format::Error>;
}

#[sealed]
impl<T> TryIntoFailureMessage for T
where
    T: IsCanceled + ToString,
{
    fn try_into_failure_message(self, id: Id, subject: Subject) -> Result<Message, format::Error> {
        Ok(if self.is_canceled() {
            Message::canceled(id, subject).build()
        } else {
            Message::error(id, subject, &self.to_string())?.build()
        })
    }
}
