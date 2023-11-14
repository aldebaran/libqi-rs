use qi_value::{ActionId, Value};

#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    qi_macros::Reflect,
    qi_macros::FromValue,
    qi_macros::ToValue,
    qi_macros::IntoValue,
)]
#[serde(transparent)]
#[qi(transparent)]
pub struct SignalLink(u64);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Event {
    uid: ActionId,
    value: Value<'static>,
}
