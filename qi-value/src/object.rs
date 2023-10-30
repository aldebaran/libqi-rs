use crate::{AsValue, FromValue, FromValueError, Map, Reflect, Signature, Type, Value};

#[derive(
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
)]
#[qi(value = "crate")]
pub struct Object {
    pub meta_object: MetaObject,
    pub service_id: ServiceId,
    pub object_id: ObjectId,
    pub object_uid: ObjectUid,
}

impl Reflect for Object {
    fn ty() -> Option<Type> {
        Some(Type::Object)
    }
}

impl AsValue for Object {
    fn value_type(&self) -> Type {
        Type::Object
    }

    fn as_value(&self) -> Value<'_> {
        Value::Object(Box::new(self.clone()))
    }
}

impl FromValue<'_> for Object {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Object(object) => Ok(*object),
            _ => Err(FromValueError::TypeMismatch {
                expected: "an Object".to_owned(),
                actual: value.value_type().to_string(),
            }),
        }
    }
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "object(uid={})", &self.object_uid)
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
    qi_macros::Reflect,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::AsValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(value = "crate", transparent)]
pub struct ServiceId(pub u32);

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
    qi_macros::Reflect,
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(value = "crate", transparent)]
pub struct ObjectId(pub u32);

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
    qi_macros::Reflect,
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
#[qi(value = "crate", transparent)]
pub struct ActionId(pub u32);

impl ActionId {
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
    qi_macros::Reflect,
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
    derive_more::From,
    derive_more::Into,
    derive_more::IntoIterator,
)]
#[qi(value = "crate", transparent)]
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
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    qi_macros::Reflect,
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
)]
#[qi(value = "crate")]
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

    pub fn signal(&self, name: &str) -> Option<(&ActionId, &MetaSignal)> {
        self.signals.iter().find(|(_, sig)| sig.name == name)
    }

    pub fn method(&self, name: &str) -> Option<(&ActionId, &MetaMethod)> {
        self.methods.iter().find(|(_, method)| method.name == name)
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
        name: String,
        parameters_signature: Signature,
        return_signature: Signature,
    ) -> Self {
        self.meta_object.methods.insert(
            uid,
            MetaMethod {
                uid,
                return_signature,
                name,
                parameters_signature,
                description: String::new(),
                parameters: Vec::new(),
                return_description: String::new(),
            },
        );
        self
    }

    pub fn add_signal(mut self, uid: ActionId, name: String, signature: Signature) -> Self {
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
    qi_macros::Reflect,
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
)]
#[qi(value = "crate", rename_all = "camelCase")]
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
    qi_macros::Reflect,
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
)]
#[qi(value = "crate")]
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
    qi_macros::Reflect,
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
)]
#[qi(value = "crate")]
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
    qi_macros::Reflect,
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::StdTryFromValue,
    qi_macros::IntoValue,
    serde::Serialize,
    serde::Deserialize,
)]
#[qi(value = "crate")]
pub struct MetaProperty {
    pub uid: ActionId,
    pub name: String,
    pub signature: Signature,
}
