#[derive(
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    qi_macros::Valuable,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(value(crate = "crate", transparent))]
pub struct ServiceId(pub u32);

impl ServiceId {
    pub const DEFAULT: Self = Self(0);
}

#[derive(
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    qi_macros::Valuable,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(value(crate = "crate", transparent))]
pub struct ObjectId(pub u32);

impl ObjectId {
    pub const DEFAULT: Self = Self(0);
}

#[derive(
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    qi_macros::Valuable,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(value(crate = "crate", transparent))]
pub struct ActionId(pub u32);

impl ActionId {
    pub const DEFAULT: Self = Self(0);

    pub fn wrapping_next(&mut self) -> Self {
        let old_id = self.0;
        self.0 = self.0.wrapping_add(1);
        Self(old_id)
    }
}

impl Iterator for ActionId {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.wrapping_next())
    }
}
