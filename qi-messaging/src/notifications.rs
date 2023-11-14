use crate::{capabilities, message};
use bytes::Bytes;

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Event {
    pub(crate) address: message::Address,
    pub(crate) body: Bytes,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Capabilities<'a> {
    pub(crate) address: message::Address,
    #[serde(borrow)]
    pub(crate) capabilities: capabilities::CapabilitiesMap<'a>,
}

#[derive(Clone, PartialEq, Eq, Debug, derive_more::From, serde::Serialize, serde::Deserialize)]
pub enum Notification<'a> {
    Event(Event),
    #[serde(borrow)]
    Capabilities(Capabilities<'a>),
}
