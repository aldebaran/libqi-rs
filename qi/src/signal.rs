use qi_value::{ActionId, Value};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "crate::value", transparent)]
pub struct SignalLink(u64);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Event {
    uid: ActionId,
    value: Value<'static>,
}
