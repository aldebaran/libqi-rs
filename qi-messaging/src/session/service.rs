pub mod request;

use super::control;
use crate::message;
pub use crate::message::{Action, Object, Service};
pub use request::{Call, Cancel, Event, Post, Request};

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Subject {
    service: Service,
    object: Object,
    action: Action,
}

impl Subject {
    pub fn new(service: Service, object: Object, action: Action) -> Option<Self> {
        if control::is_control_service(service) || control::is_control_object(object) {
            None
        } else {
            Some(Self {
                service,
                object,
                action,
            })
        }
    }

    pub fn service(&self) -> Service {
        self.service
    }

    pub fn object(&self) -> Object {
        self.object
    }

    pub fn action(&self) -> Action {
        self.action
    }

    pub(crate) fn from_message(subject: message::Subject) -> Option<Self> {
        Self::new(subject.service(), subject.object(), subject.action())
    }

    pub(crate) fn into_message(self) -> message::Subject {
        message::Subject::new(self.service, self.object, self.action)
    }
}

impl From<Subject> for message::Subject {
    fn from(subject: Subject) -> Self {
        subject.into_message()
    }
}
