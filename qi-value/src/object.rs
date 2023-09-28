use crate::{struct_ty, ty, Map, Signature, Type};

#[derive(Clone, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Object {
    pub meta_object: MetaObject,
    pub service_id: ServiceId,
    pub object_id: ObjectId,
    pub object_uid: ObjectUid,
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "object(uid={})", &self.object_uid)
    }
}

impl ty::StaticGetType for Object {
    fn static_type() -> Type {
        Type::Object
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
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
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
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
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
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
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
    derive_more::From,
    derive_more::Into,
    derive_more::IntoIterator,
)]
pub struct ObjectUid([u32; 5]); // SHA-1 digest

impl ObjectUid {
    pub const SIZE: usize = 20;

    pub const fn new(digest: [u32; 5]) -> Self {
        Self(digest)
    }
}

impl std::fmt::Display for ObjectUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [h0, h1, h2, h3, h4] = &self.0;
        write!(f, "{h0:x}-{h1:x}-{h2:x}-{h3:x}-{h4:x}",)
    }
}

impl serde::Serialize for ObjectUid {
    // SHA-1 parts are always serialized as big endian.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(20)?;
        use serde::ser::SerializeTuple;
        for dword in self.0 {
            for byte in dword.to_be_bytes() {
                tuple.serialize_element(&byte)?;
            }
        }
        tuple.end()
    }
}

impl<'de> serde::Deserialize<'de> for ObjectUid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let buf = <[u8; 20]>::deserialize(deserializer)?;
        let mut digest = [0u32; 5];
        for (index, dword) in digest.iter_mut().enumerate() {
            let offset = index * 4;
            *dword = u32::from_be_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
            ]);
        }
        Ok(Self(digest))
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MetaObject {
    pub methods: Map<ActionId, MetaMethod>,
    pub signals: Map<ActionId, MetaSignal>,
    pub properties: Map<ActionId, MetaProperty>,
    pub description: String,
}

impl MetaObject {
    pub fn builder() -> MetaObjectBuilder {
        MetaObjectBuilder::new()
    }
}

impl ty::StaticGetType for MetaObject {
    fn static_type() -> Type {
        struct_ty! {
            MetaObject {
                methods: ty::map_of(
                    Type::UInt32, MetaMethod::static_type()
                ),
                signals: ty::map_of(
                    Type::UInt32, MetaSignal::static_type()
                ),
                properties: ty::map_of(
                    Type::UInt32, MetaProperty::static_type()
                ),
                description: Type::String,
            }
        }
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
        &mut self,
        uid: ActionId,
        name: impl Into<String>,
        parameters_signature: impl Into<Signature>,
        return_signature: impl Into<Signature>,
    ) -> ActionId {
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
        uid
    }

    pub fn add_signal(
        &mut self,
        uid: ActionId,
        name: impl Into<String>,
        signature: impl Into<Signature>,
    ) -> ActionId {
        self.meta_object.signals.insert(
            uid,
            MetaSignal {
                uid,
                name: name.into(),
                signature: signature.into(),
            },
        );
        uid
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
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaMethod {
    pub uid: ActionId,
    pub return_signature: Signature,
    pub name: String,
    pub parameters_signature: Signature,
    pub description: String,
    pub parameters: Vec<MetaMethodParameter>,
    pub return_description: String,
}

impl ty::StaticGetType for MetaMethod {
    fn static_type() -> Type {
        struct_ty! {
            MetaMethod {
                uid: Type::UInt32,
                returnSignature: Type::String,
                name: Type::String,
                parametersSignature: Type::String,
                description: Type::String,
                parameters: ty::list_of(MetaMethodParameter::static_type()),
                returnDescription: Type::String,
            }
        }
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
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaMethodParameter {
    pub name: String,
    pub description: String,
}

impl ty::StaticGetType for MetaMethodParameter {
    fn static_type() -> Type {
        struct_ty! {
            MetaMethodParameter {
                name: Type::String,
                description: Type::String,
            }
        }
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
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaSignal {
    pub uid: ActionId,
    pub name: String,
    pub signature: Signature,
}

impl ty::StaticGetType for MetaSignal {
    fn static_type() -> Type {
        struct_ty! {
            MetaSignal {
                uid: Type::UInt32,
                name: Type::String,
                signature: Type::String,
            }
        }
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
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MetaProperty {
    pub uid: ActionId,
    pub name: String,
    pub signature: Signature,
}

impl ty::StaticGetType for MetaProperty {
    fn static_type() -> Type {
        struct_ty! {
            MetaProperty {
                uid: Type::UInt32,
                name: Type::String,
                signature: Type::String,
            }
        }
    }
}
