use super::control;
use crate::{
    message::{self, Action, Object, Service},
    request::Id,
};
use bytes::Bytes;

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Subject {
    service: Service,
    object: Object,
    action: Action,
}

impl Subject {
    pub(crate) fn from_message(subject: message::Subject) -> Option<Self> {
        if control::is_control_service(subject.service())
            || control::is_control_object(subject.object())
        {
            None
        } else {
            Some(Self {
                service: subject.service(),
                object: subject.object(),
                action: subject.action(),
            })
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::From)]
pub enum Request {
    Call(Call),
    Post(Post),
    Event(Event),
    Cancel(Cancel),
}

impl Request {
    pub(crate) fn try_from_messaging(
        request: crate::request::Request,
    ) -> Result<Self, crate::request::Request> {
        Ok(match request {
            crate::request::Request::Call(call) => Call::try_from_messaging(call)?.into(),
            crate::request::Request::Post(post) => Post::try_from_messaging(post)?.into(),
            crate::request::Request::Event(event) => Event::try_from_messaging(event)?.into(),
            crate::request::Request::Cancel(cancel) => Cancel::try_from_messaging(cancel)?.into(),
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
    pub(crate) fn try_from_messaging(
        call: crate::request::Call,
    ) -> Result<Self, crate::request::Call> {
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
    pub(crate) fn try_from_messaging(
        post: crate::request::Post,
    ) -> Result<Self, crate::request::Post> {
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
    pub(crate) fn try_from_messaging(
        event: crate::request::Event,
    ) -> Result<Self, crate::request::Event> {
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
    pub(crate) fn try_from_messaging(
        cancel: crate::request::Cancel,
    ) -> Result<Self, crate::request::Cancel> {
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
