mod capabilities;
pub(super) mod client;
// mod server;

use crate::{
    capabilities::Map as Capabilities,
    channel::request::Request as ChannelRequest,
    format,
    message::{self, Action},
    request::{CallResult, Request as MessagingRequest, Response as MessagingResponse},
};

pub(super) const CONTROL_SERVICE: message::Service = message::Service::new(0);
pub(super) const CONTROL_OBJECT: message::Object = message::Object::new(0);

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
pub(crate) struct Subject(Action);

impl Subject {
    pub(super) fn from_message_subject(subject: message::Subject) -> Option<Self> {
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

const CAPABILITIES_SUBJECT: Subject = Subject(Action::new(0));
const AUTHENTICATE_SUBJECT: Subject = Subject(Action::new(8));

// pub(crate) async fn send_capabilities(channel: &mut Channel) -> Result<(), channel::Error> {
//     let request = channel::Request::Capabilities(capabilities::local().clone());
//     async { channel.ready().await?.call(request).await }.await?;
//     Ok(())
// }

#[derive(Clone, Debug)]
pub(super) enum Request {
    Authenticate(Capabilities), // TODO: use a reference to the map.
    UpdateCapabilities(Capabilities),
}

impl Request {
    pub(super) fn try_from_messaging_request(
        request: MessagingRequest,
    ) -> Result<Result<Self, MessagingRequest>, format::Error> {
        Ok(match request {
            MessagingRequest::Call {
                subject, payload, ..
            } if subject == AUTHENTICATE_SUBJECT => {
                let capabilities = format::from_bytes(&payload)?;
                Ok(Self::Authenticate(capabilities))
            }
            MessagingRequest::Capabilities { capabilities, .. } => {
                Ok(Self::UpdateCapabilities(capabilities))
            }
            _ => Err(request),
        })
    }

    fn try_into_channel_request(self) -> Result<ChannelRequest, format::Error> {
        match self {
            Request::Authenticate(capabilities) => {
                ChannelRequest::call(AUTHENTICATE_SUBJECT.into(), &capabilities)
            }
            Request::UpdateCapabilities(capabilities) => Ok(ChannelRequest::Capabilities {
                subject: CAPABILITIES_SUBJECT.into(),
                capabilities,
            }),
        }
    }
}

#[derive(Clone, Debug, derive_more::Into, derive_more::From)]
pub(super) struct Response(Option<Capabilities>);

impl Response {
    pub(super) fn try_into_messaging_response(self) -> Result<MessagingResponse, format::Error> {
        Ok(self
            .0
            .map(|capabilities| CallResult::reply(&capabilities))
            .transpose()?
            .into())
    }
}
