use qi_type::{Signature, Type, Typed};
use std::collections::HashMap;

#[derive(Clone, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Object {
    pub meta_object: MetaObject,
    pub service_id: ServiceId,
    pub object_id: ObjectId,
    pub object_uid: ObjectUid,
}

impl Typed for Object {
    fn ty() -> Option<Type> {
        Some(Type::Object)
    }
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "object(uid={})", &self.object_uid)
    }
}

pub(crate) fn deserialize_object<'de, D, V>(
    deserializer: D,
    visitor: V,
) -> Result<V::Value, D::Error>
where
    D: serde::Deserializer<'de>,
    V: serde::de::Visitor<'de>,
{
    deserializer.deserialize_struct(
        "Object",
        &["meta_object", "service_id", "object_id", "object_uid"],
        visitor,
    )
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
    qi_derive::Typed,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(typed(transparent))]
pub struct ServiceId(u32);

impl ServiceId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
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
    qi_derive::Typed,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(typed(transparent))]
pub struct ObjectId(u32);

impl ObjectId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
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
    qi_derive::Typed,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(typed(transparent))]
pub struct ActionId(u32);

impl ActionId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn incr(&mut self) -> Self {
        let old_id = self.0;
        self.0 = self.0.wrapping_add(1);
        Self(old_id)
    }
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
    qi_derive::Typed,
    serde::Serialize,
    serde::Deserialize,
    derive_more::From,
    derive_more::Into,
    derive_more::IntoIterator,
)]
#[qi(typed(transparent))]
pub struct ObjectUid(
    // SHA-1 digest as bytes of Big Endian encoded sequence of 5 DWORD.
    [u8; 20],
);

impl ObjectUid {
    pub fn from_digest(digest: [u32; 5]) -> Self {
        let mut bytes = <[u8; 20]>::default();
        for (src, dst) in digest.iter().zip(bytes.chunks_exact_mut(4)) {
            dst.copy_from_slice(&src.to_be_bytes())
        }
        Self(bytes)
    }

    pub const fn bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl std::fmt::Display for ObjectUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, bytes) in self.0.chunks_exact(4).enumerate() {
            if i > 0 {
                write!(f, "-")?;
            }
            let dword = u32::from_be_bytes(bytes.try_into().unwrap());
            write!(f, "{dword:x}")?;
        }
        Ok(())
    }
}

#[derive(
    Clone, Default, PartialEq, Eq, Debug, qi_derive::Typed, serde::Serialize, serde::Deserialize,
)]
pub struct MetaObject {
    pub methods: HashMap<ActionId, MetaMethod>,
    pub signals: HashMap<ActionId, MetaSignal>,
    pub properties: HashMap<ActionId, MetaProperty>,
    pub description: String,
}

impl MetaObject {
    pub fn builder() -> MetaObjectBuilder {
        MetaObjectBuilder::new()
    }
}

#[derive(Default, Debug)]
pub struct MetaObjectBuilder {
    meta_object: MetaObject,
}

impl MetaObjectBuilder {
    pub fn new() -> Self {
        Self {
            meta_object: Default::default(),
        }
    }

    pub fn add_method(
        mut self,
        uid: ActionId,
        name: impl Into<String>,
        parameters_signature: impl Into<Signature>,
        return_signature: impl Into<Signature>,
    ) -> Self {
        self.meta_object.methods.insert(
            uid,
            MetaMethod {
                uid,
                return_signature: return_signature.into(),
                name: name.into(),
                parameters_signature: parameters_signature.into(),
                description: String::new(),
                parameters: Vec::new(),
                return_description: String::new(),
            },
        );
        self
    }

    pub fn add_signal(
        mut self,
        uid: ActionId,
        name: impl Into<String>,
        signature: impl Into<Signature>,
    ) -> Self {
        self.meta_object.signals.insert(
            uid,
            MetaSignal {
                uid,
                name: name.into(),
                signature: signature.into(),
            },
        );
        self
    }

    pub fn add_property(
        mut self,
        uid: ActionId,
        name: impl Into<String>,
        signature: impl Into<Signature>,
    ) -> Self {
        let name = name.into();
        let signature = signature.into();
        self.meta_object.properties.insert(
            uid,
            MetaProperty {
                uid,
                name: name.clone(),
                signature: signature.clone(),
            },
        );
        // Properties are also signals
        self.meta_object.signals.insert(
            uid,
            MetaSignal {
                uid,
                name,
                signature,
            },
        );
        self
    }

    pub fn build(self) -> MetaObject {
        self.meta_object
    }
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
    qi_derive::Typed,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "camelCase")]
#[qi(typed(rename_all = "camelCase"))]
pub struct MetaMethod {
    pub uid: ActionId,
    pub return_signature: Signature,
    pub name: String,
    pub parameters_signature: Signature,
    pub description: String,
    pub parameters: Vec<MetaMethodParameter>,
    pub return_description: String,
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
    qi_derive::Typed,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaMethodParameter {
    pub name: String,
    pub description: String,
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
    qi_derive::Typed,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaSignal {
    pub uid: ActionId,
    pub name: String,
    pub signature: Signature,
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
    qi_derive::Typed,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaProperty {
    pub uid: ActionId,
    pub name: String,
    pub signature: Signature,
}
