pub(crate) use crate::request::Response;
use crate::{
    message::{self, Action, Object, Service},
    request::{Id, Request as MessagingRequest},
};
use bytes::Bytes;

use super::control::{CONTROL_OBJECT, CONTROL_SERVICE};

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Subject {
    service: Service,
    object: Object,
    action: Action,
}

impl Subject {
    pub(crate) fn from_message_subject(subject: message::Subject) -> Option<Self> {
        if subject.service() == CONTROL_SERVICE || subject.object() == CONTROL_OBJECT {
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Request {
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
}

impl Request {
    pub(crate) fn try_from_messaging_request(
        request: MessagingRequest,
    ) -> Result<Self, MessagingRequest> {
        use MessagingRequest::*;
        let subject = match Subject::from_message_subject(request.subject()) {
            Some(subject) => subject,
            None => return Err(request),
        };
        Ok(match request {
            Call { id, payload, .. } => Request::Call {
                id,
                subject,
                payload,
            },
            Post { id, payload, .. } => Request::Post {
                id,
                subject,
                payload,
            },
            Event { id, payload, .. } => Request::Event {
                id,
                subject,
                payload,
            },
            Cancel { id, call_id, .. } => Request::Cancel {
                id,
                subject,
                call_id,
            },
            _ => return Err(request),
        })
    }
}
