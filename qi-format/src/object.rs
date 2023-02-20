use crate::{Signature, String};
use indexmap::IndexMap;

#[derive(
    Clone,
    Default,
    PartialEq,
    Eq,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Object<'o> {
    #[serde(borrow)]
    meta_object: MetaObject<'o>,
    service_id: u32,
    object_id: u32,
    object_uid: [u32; 5], // SHA-1 digest
}

impl<'o> std::hash::Hash for Object<'o> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        todo!()
    }
}

impl<'o> std::fmt::Display for Object<'o> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(
    Clone,
    Default,
    PartialEq,
    Eq,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaObject<'o> {
    #[serde(borrow)]
    methods: IndexMap<u32, MetaMethod<'o>>,
    #[serde(borrow)]
    signals: IndexMap<u32, MetaSignal<'o>>,
    #[serde(borrow)]
    properties: IndexMap<u32, MetaProperty<'o>>,
    #[serde(borrow)]
    description: String<'o>,
}

#[derive(
    Clone,
    Default,
    PartialEq,
    Eq,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaMethod<'m> {
    uid: u32,
    return_signature: Signature,
    #[serde(borrow)]
    name: String<'m>,
    parameters_signature: Signature,
    #[serde(borrow)]
    description: String<'m>,
    #[serde(borrow)]
    parameters: IndexMap<String<'m>, String<'m>>,
    #[serde(borrow)]
    return_description: String<'m>,
}

#[derive(
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaSignal<'s> {
    uid: u32,
    signature: Signature,
    #[serde(borrow)]
    name: String<'s>,
}

#[derive(
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaProperty<'p> {
    uid: u32,
    signature: Signature,
    #[serde(borrow)]
    name: String<'p>,
}
