pub mod client;
pub mod server;

use crate::{
    capabilities,
    channel::{self, Channel},
    format,
    message::{self, Action},
};
use tower::{Service, ServiceExt};

const AUTHENTICATE_SUBJECT: message::Subject = message::Subject::control(Action::new(8));

pub(crate) async fn send_capabilities(channel: &mut Channel) -> Result<(), channel::Error> {
    let request = channel::Request::Capabilities(capabilities::local().clone());
    async { channel.ready().await?.call(request).await }.await?;
    Ok(())
}

#[derive(Clone, Debug)]
pub enum Request {
    Authenticate(capabilities::Map), // TODO: use a reference to the map.
    UpdateCapabilities(capabilities::Map),
}

impl Request {
    fn try_into_channel_request(self) -> Result<channel::Request, format::Error> {
        match self {
            Request::Authenticate(capabilities) => {
                channel::Request::call(AUTHENTICATE_SUBJECT, &capabilities)
            }
            Request::UpdateCapabilities(capabilities) => {
                Ok(channel::Request::Capabilities(capabilities))
            }
        }
    }
}

impl TryFrom<channel::Request> for Option<Request> {
    type Error = format::Error;
    fn try_from(request: channel::Request) -> Result<Self, Self::Error> {
        Ok(match request {
            channel::Request::Call { subject, payload } if subject == AUTHENTICATE_SUBJECT => {
                let capabilities = format::from_bytes(&payload)?;
                Some(Request::Authenticate(capabilities))
            }
            channel::Request::Capabilities(capabilities) => {
                Some(Request::UpdateCapabilities(capabilities))
            }
            _ => None,
        })
    }
}

#[derive(Clone, Debug, derive_more::Into, derive_more::From)]
pub struct Response(Option<capabilities::Map>);

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("error updating capabilities")]
    UpdateCapabilities(#[source] capabilities::ExpectedKeyValueError<bool>),
}
