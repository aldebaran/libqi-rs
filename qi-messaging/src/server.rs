use crate::{
    capabilities, format,
    message::{self, Action, Message},
};
use sealed::sealed;

const AUTHENTICATE_SUBJECT: message::Subject = message::Subject::server(Action::new(8));

#[sealed]
pub(crate) trait MessageBuilderExt {
    fn server_authenticate(
        self,
        id: message::Id,
        capabilities: &capabilities::Map,
    ) -> Result<Self, format::Error>
    where
        Self: Sized;
}

#[sealed]
impl MessageBuilderExt for message::Builder {
    fn server_authenticate(
        self,
        id: message::Id,
        capabilities: &capabilities::Map,
    ) -> Result<Self, format::Error> {
        self.set_id(id)
            .set_kind(message::Kind::Call)
            .set_subject(AUTHENTICATE_SUBJECT)
            .set_value(&capabilities)
    }
}

#[sealed]
pub(crate) trait MessageExt {
    /// Returns true if the message is a messaging server authentication message.
    /// No check is done on the type of the message.
    fn is_server_authenticate(&self) -> bool;
}

#[sealed]
impl MessageExt for Message {
    fn is_server_authenticate(&self) -> bool {
        self.subject() == AUTHENTICATE_SUBJECT
    }
}
