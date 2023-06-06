pub(super) mod capabilities;
pub(super) mod client;
pub(super) mod request;
// mod server;

use crate::message::{self, Action};

const CONTROL_SERVICE: message::Service = message::Service::new(0);
const CONTROL_OBJECT: message::Object = message::Object::new(0);

pub(in crate::session) fn is_control_service(service: message::Service) -> bool {
    service == CONTROL_SERVICE
}

pub(in crate::session) fn is_control_object(object: message::Object) -> bool {
    object == CONTROL_OBJECT
}

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
pub(in crate::session) struct Subject(Action);

impl Subject {
    pub(in crate::session) fn from_message_subject(subject: message::Subject) -> Option<Self> {
        match (subject.service(), subject.object()) {
            (CONTROL_SERVICE, CONTROL_OBJECT) => Some(Self(subject.action())),
            _ => None,
        }
    }
}

impl From<Subject> for message::Subject {
    fn from(subject: Subject) -> Self {
        Self::new(CONTROL_SERVICE, CONTROL_OBJECT, subject.0)
    }
}

impl PartialEq<message::Subject> for Subject {
    fn eq(&self, other: &message::Subject) -> bool {
        other.service() == CONTROL_SERVICE
            && other.object() == CONTROL_OBJECT
            && other.action() == self.0
    }
}

impl PartialEq<Subject> for message::Subject {
    fn eq(&self, other: &Subject) -> bool {
        other == self
    }
}

// const std::string AuthProvider::QiAuthPrefix     = "__qi_auth_";
// const std::string AuthProvider::UserAuthPrefix   = "auth_";
// const std::string AuthProvider::Error_Reason_Key = QiAuthPrefix + "err_reason";
// const std::string AuthProvider::State_Key        = QiAuthPrefix + "state";

// pub(crate) async fn send_capabilities(channel: &mut Channel) -> Result<(), channel::Error> {
//     let request = channel::Request::Capabilities(capabilities::local().clone());
//     async { channel.ready().await?.call(request).await }.await?;
//     Ok(())
// }
