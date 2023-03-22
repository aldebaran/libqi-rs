use crate::{struct_ty, ty, List, Map, Signature, Type};

#[derive(Clone, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Object {
    pub meta_object: MetaObject,
    pub service_id: u32,
    pub object_id: u32,
    #[serde(with = "serde_sha1")]
    pub object_uid: [u32; 5], // SHA-1 digest
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [h0, h1, h2, h3, h4] = &self.object_uid;
        write!(f, "object(uid={h0:x}-{h1:x}-{h2:x}-{h3:x}-{h4:x})",)
    }
}

impl ty::StaticGetType for Object {
    fn ty() -> Type {
        Type::Object
    }
}

mod serde_sha1 {
    // SHA-1 parts are always serialized as big endian.
    pub fn serialize<S>(digest: &[u32; 5], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(20)?;
        use serde::ser::SerializeTuple;
        for dword in digest {
            for byte in dword.to_be_bytes() {
                tuple.serialize_element(&byte)?;
            }
        }
        tuple.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u32; 5], D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
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
        Ok(digest)
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MetaObject {
    pub methods: Map<u32, MetaMethod>,
    pub signals: Map<u32, MetaSignal>,
    pub properties: Map<u32, MetaProperty>,
    pub description: String,
}

impl ty::StaticGetType for MetaObject {
    fn ty() -> Type {
        struct_ty! {
            MetaObject {
                methods: ty::map_of(
                    Type::UInt32, MetaMethod::ty()
                ),
                signals: ty::map_of(
                    Type::UInt32, MetaSignal::ty()
                ),
                properties: ty::map_of(
                    Type::UInt32, MetaProperty::ty()
                ),
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
pub struct MetaMethod {
    pub uid: u32,
    pub return_signature: Signature,
    pub name: String,
    pub parameters_signature: Signature,
    pub description: String,
    pub parameters: List<MetaMethodParameter>,
    pub return_description: String,
}

impl ty::StaticGetType for MetaMethod {
    fn ty() -> Type {
        struct_ty! {
            MetaMethod {
                uid: Type::UInt32,
                returnSignature: Type::String,
                name: Type::String,
                parametersSignature: Type::String,
                description: Type::String,
                parameters: ty::list_of(MetaMethodParameter::ty()),
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
    fn ty() -> Type {
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
    pub uid: u32,
    pub name: String,
    pub signature: Signature,
}

impl ty::StaticGetType for MetaSignal {
    fn ty() -> Type {
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
    pub uid: u32,
    pub name: String,
    pub signature: Signature,
}

impl ty::StaticGetType for MetaProperty {
    fn ty() -> Type {
        struct_ty! {
            MetaProperty {
                uid: Type::UInt32,
                name: Type::String,
                signature: Type::String,
            }
        }
    }
}
