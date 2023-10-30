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
    qi_macros::Typed,
)]
#[serde(transparent)]
#[qi(typed(transparent))]
pub struct SignalLink(u64);
