use super::{Action, Subject};
use crate::{channel, format};

const CAPABILITIES_SUBJECT: Subject = Subject(Action::new(0));
const AUTHENTICATE_SUBJECT: Subject = Subject(Action::new(8));

#[derive(Debug, derive_more::From)]
pub(in crate::session) enum Request {
    Authenticate(Authenticate),
    UpdateCapabilities(UpdateCapabilities),
}

impl Request {
    pub(in crate::session) fn try_from_messaging(
        request: crate::request::Request,
    ) -> Result<Result<Self, crate::request::Request>, format::Error> {
        Ok(match request {
            crate::request::Request::Call(call) if call.subject == AUTHENTICATE_SUBJECT => {
                Ok(Authenticate(call.value()?).into())
            }
            crate::request::Request::Capabilities(capabilities) => {
                Ok(UpdateCapabilities(capabilities.into()).into())
            }
            _ => Err(request),
        })
    }

    fn try_into_channel(self) -> Result<channel::request::Request, format::Error> {
        Ok(match self {
            Request::Authenticate(authenticate) => {
                channel::request::Call::try_from(authenticate)?.into()
            }
            Request::UpdateCapabilities(update_capabilities) => {
                channel::request::Capabilities::from(update_capabilities).into()
            }
        })
    }
}

#[derive(Debug, derive_more::Into)]
pub(in crate::session) struct Authenticate(crate::capabilities::Map);

impl Authenticate {
    pub(in crate::session) fn new() -> Self {
        Self(super::capabilities::local().clone())
    }
}

impl TryFrom<Authenticate> for channel::request::Call {
    type Error = format::Error;

    fn try_from(auth: Authenticate) -> Result<Self, Self::Error> {
        channel::request::Call::with_value(AUTHENTICATE_SUBJECT.into(), &auth.0)
    }
}

#[derive(Debug, derive_more::Into)]
pub(in crate::session) struct UpdateCapabilities(crate::capabilities::Map);

impl From<UpdateCapabilities> for channel::request::Capabilities {
    fn from(update_capabilities: UpdateCapabilities) -> Self {
        Self::new(CAPABILITIES_SUBJECT.into(), update_capabilities.0)
    }
}
