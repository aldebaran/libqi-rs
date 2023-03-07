use crate::{annotated_tuple_type, map_type, typing, List, Map, Signature, String, Type, UInt32};

#[derive(Clone, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Object {
    meta_object: MetaObject,
    service_id: UInt32,
    object_id: UInt32,
    #[serde(with = "serde_sha1")]
    object_uid: [UInt32; 5], // SHA-1 digest
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [h0, h1, h2, h3, h4] = &self.object_uid;
        write!(f, "object(uid={h0:x}-{h1:x}-{h2:x}-{h3:x}-{h4:x})",)
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
    methods: Map<UInt32, MetaMethod>,
    signals: Map<UInt32, MetaSignal>,
    properties: Map<UInt32, MetaProperty>,
    description: String,
}

impl MetaObject {
    pub fn get_type() -> Type {
        annotated_tuple_type! {
            "MetaObject" => {
                "methods" => map_type! {
                    Type::UInt32 => MetaMethod::get_type()
                },
                "signals" => map_type! {
                    Type::UInt32 => MetaSignal::get_type()
                },
                "properties" => map_type! {
                    Type::UInt32 => MetaProperty::get_type()
                },
                "description" => Type::String,
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
    uid: UInt32,
    return_signature: Signature,
    name: String,
    parameters_signature: Signature,
    description: String,
    parameters: List<MetaMethodParameter>,
    return_description: String,
}

impl MetaMethod {
    pub fn get_type() -> Type {
        annotated_tuple_type! {
            "MetaMethod" => {
                "uid" => Type::UInt32,
                "returnSignature" => Type::String,
                "name" => Type::String,
                "parametersSignature" => Type::String,
                "description" => Type::String,
                "parameters" => typing::list(MetaMethodParameter::get_type()),
                "returnDescription" => Type::String,
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
    name: String,
    description: String,
}

impl MetaMethodParameter {
    pub fn get_type() -> Type {
        annotated_tuple_type! {
            "MetaMethodParameter" => {
                "name" => Type::String,
                "description" => Type::String,
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
    uid: u32,
    name: String,
    signature: Signature,
}

impl MetaSignal {
    pub fn get_type() -> Type {
        annotated_tuple_type! {
            "MetaSignal" => {
                "uid" => Type::UInt32,
                "name" => Type::String,
                "signature" => Type::String,
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
    uid: u32,
    name: String,
    signature: Signature,
}

impl MetaProperty {
    pub fn get_type() -> Type {
        annotated_tuple_type! {
            "MetaProperty" => {
                "uid" => Type::UInt32,
                "name" => Type::String,
                "signature" => Type::String,
            }
        }
    }
}
