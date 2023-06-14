use super::Subject;
use crate::format;
pub use crate::message::Id;
pub use bytes::Bytes;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::From)]
pub enum Request {
    Call(Call),
    Post(Post),
    Event(Event),
    Cancel(Cancel),
}

impl Request {
    pub fn id(&self) -> Id {
        match self {
            Request::Call(call) => call.id(),
            Request::Post(post) => post.id(),
            Request::Event(event) => event.id(),
            Request::Cancel(cancel) => cancel.id(),
        }
    }

    pub fn subject(&self) -> Subject {
        match self {
            Request::Call(call) => call.subject(),
            Request::Post(post) => post.subject(),
            Request::Event(event) => event.subject(),
            Request::Cancel(cancel) => cancel.subject(),
        }
    }

    pub(crate) fn try_from_messaging(request: crate::Request) -> Result<Self, crate::Request> {
        Ok(match request {
            crate::Request::Call(call) => Call::try_from_messaging(call)?.into(),
            crate::Request::Post(post) => Post::try_from_messaging(post)?.into(),
            crate::Request::Event(event) => Event::try_from_messaging(event)?.into(),
            crate::Request::Cancel(cancel) => Cancel::try_from_messaging(cancel)?.into(),
            _ => return Err(request),
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Call {
    id: Id,
    subject: Subject,
    payload: Bytes,
}

impl Call {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    pub fn value<'de, T>(&'de self) -> Result<T, format::Error>
    where
        T: serde::Deserialize<'de>,
    {
        format::from_bytes(&self.payload)
    }

    pub(crate) fn try_from_messaging(call: crate::Call) -> Result<Self, crate::Call> {
        let subject = match Subject::from_message(call.subject) {
            Some(subject) => subject,
            None => return Err(call),
        };
        Ok(Self {
            id: call.id,
            subject,
            payload: call.payload,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Post {
    id: Id,
    subject: Subject,
    payload: Bytes,
}

impl Post {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    pub fn value<'de, T>(&'de self) -> Result<T, format::Error>
    where
        T: serde::Deserialize<'de>,
    {
        format::from_bytes(&self.payload)
    }

    pub(crate) fn try_from_messaging(post: crate::Post) -> Result<Self, crate::Post> {
        let subject = match Subject::from_message(post.subject) {
            Some(subject) => subject,
            None => return Err(post),
        };
        Ok(Self {
            id: post.id,
            subject,
            payload: post.payload,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Event {
    id: Id,
    subject: Subject,
    payload: Bytes,
}

impl Event {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    pub fn value<'de, T>(&'de self) -> Result<T, format::Error>
    where
        T: serde::Deserialize<'de>,
    {
        format::from_bytes(&self.payload)
    }

    pub(crate) fn try_from_messaging(event: crate::Event) -> Result<Self, crate::Event> {
        let subject = match Subject::from_message(event.subject) {
            Some(subject) => subject,
            None => return Err(event),
        };
        Ok(Self {
            id: event.id,
            subject,
            payload: event.payload,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Cancel {
    id: Id,
    subject: Subject,
    call_id: Id,
}

impl Cancel {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn call_id(&self) -> Id {
        self.call_id
    }

    pub(crate) fn try_from_messaging(cancel: crate::Cancel) -> Result<Self, crate::Cancel> {
        let subject = match Subject::from_message(cancel.subject) {
            Some(subject) => subject,
            None => return Err(cancel),
        };
        Ok(Self {
            id: cancel.id,
            subject,
            call_id: cancel.call_id,
        })
    }
}
