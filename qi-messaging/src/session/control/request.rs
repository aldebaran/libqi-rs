use super::{Action, Subject};
use crate::{channel, format, CapabilitiesMap};

const CAPABILITIES_SUBJECT: Subject = Subject(Action::new(0));
const AUTHENTICATE_SUBJECT: Subject = Subject(Action::new(8));

#[derive(Debug, derive_more::From)]
pub(in crate::session) enum Request {
    Authenticate(Authenticate),
    UpdateCapabilities(UpdateCapabilities),
}

impl Request {
    pub(in crate::session) fn try_from_messaging(
        request: crate::Request,
    ) -> Result<Result<Self, crate::Request>, format::Error> {
        Ok(match request {
            crate::Request::Call(call) if call.subject == AUTHENTICATE_SUBJECT => {
                Ok(Authenticate(call.value()?).into())
            }
            crate::Request::Capabilities(capabilities) => {
                Ok(UpdateCapabilities(capabilities.into()).into())
            }
            _ => Err(request),
        })
    }
}

#[derive(Debug, derive_more::Into)]
pub(in crate::session) struct Authenticate(CapabilitiesMap);

impl Authenticate {
    pub(in crate::session) fn new() -> Self {
        Self(super::capabilities::local().clone())
    }

    pub(super) fn parameters(&self) -> &CapabilitiesMap {
        &self.0
    }
}

impl TryFrom<Authenticate> for channel::as_service::Call {
    type Error = format::Error;

    fn try_from(auth: Authenticate) -> Result<Self, Self::Error> {
        channel::as_service::Call::with_value(AUTHENTICATE_SUBJECT.into(), &auth.0)
    }
}

#[derive(Debug, derive_more::Into)]
pub(in crate::session) struct UpdateCapabilities(CapabilitiesMap);

impl UpdateCapabilities {
    pub(super) fn remote_map(&self) -> &CapabilitiesMap {
        &self.0
    }
}

impl From<UpdateCapabilities> for channel::as_service::Capabilities {
    fn from(update_capabilities: UpdateCapabilities) -> Self {
        Self::new(CAPABILITIES_SUBJECT.into(), update_capabilities.0)
    }
}
