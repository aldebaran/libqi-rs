use crate::{
    ty, ActionId, FromValue, FromValueError, IntoValue, Map, ObjectId, Reflect, ServiceId,
    Signature, Type, Value,
};

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

impl<'a> IntoValue<'a> for Object {
    fn into_value(self) -> Value<'a> {
        Value::Object(Box::new(self))
    }
}

impl FromValue<'_> for Object {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Object(object) => Ok(*object),
            _ => Err(FromValueError::TypeMismatch {
                expected: "an Object".to_owned(),
                actual: value.to_string(),
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
    qi_macros::Valuable,
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

    pub fn from_bytes(bytes: [u8; 20]) -> Self {
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
    qi_macros::Valuable,
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

    pub fn signal(&self, address: &MemberAddress) -> Option<(&ActionId, &MetaSignal)> {
        self.signals.iter().find(|(sig_id, sig)| match address {
            MemberAddress::Id(id) => *sig_id == id,
            MemberAddress::Name(name) => &sig.name == name,
        })
    }

    pub fn property(&self, address: &MemberAddress) -> Option<(&ActionId, &MetaProperty)> {
        self.properties
            .iter()
            .find(|(prop_id, prop)| match address {
                MemberAddress::Id(id) => *prop_id == id,
                MemberAddress::Name(name) => &prop.name == name,
            })
    }

    pub fn method(&self, address: &MemberAddress) -> Option<(&ActionId, &MetaMethod)> {
        self.methods
            .iter()
            .find(|(method_id, method)| match address {
                MemberAddress::Id(id) => *method_id == id,
                MemberAddress::Name(name) => &method.name == name,
            })
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

    pub fn add_method(&mut self, method: MetaMethod) -> &mut Self {
        self.meta_object.methods.insert(method.uid, method);
        self
    }

    pub fn add_signal(&mut self, signal: MetaSignal) -> &mut Self {
        self.meta_object.signals.insert(signal.uid, signal);
        self
    }

    pub fn add_property(&mut self, property: MetaProperty) -> &mut Self {
        let uid = property.uid;
        self.meta_object.properties.insert(uid, property.clone());
        // Properties are also signals
        self.meta_object.signals.insert(
            uid,
            MetaSignal {
                uid,
                name: property.name,
                signature: property.signature,
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
    qi_macros::Valuable,
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

impl MetaMethod {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder<T: Into<ActionId>>(uid: T) -> MetaMethodBuilder {
        MetaMethodBuilder {
            uid: uid.into(),
            name: Default::default(),
            description: Default::default(),
            return_value: Default::default(),
            parameters: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct MetaMethodBuilder {
    uid: ActionId,
    name: String,
    description: String,
    return_value: MetaMethodBuilderReturnValue,
    parameters: Vec<MetaMethodBuilderParameter>,
}

impl MetaMethodBuilder {
    pub fn uid(&self) -> ActionId {
        self.uid
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) -> &mut Self {
        self.name = name.into();
        self
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) -> &mut Self {
        self.description = description.into();
        self
    }

    pub fn return_value(&mut self) -> &mut MetaMethodBuilderReturnValue {
        &mut self.return_value
    }

    pub fn parameter(&mut self, index: usize) -> &mut MetaMethodBuilderParameter {
        if self.parameters.len() <= index {
            self.parameters.resize_with(index + 1, Default::default);
        }
        &mut self.parameters[index]
    }

    pub fn build(self) -> MetaMethod {
        let (parameters, parameter_types) = self
            .parameters
            .into_iter()
            .map(|parameter| (parameter.parameter, parameter.ty))
            .unzip();
        let parameters_tuple = ty::Type::Tuple(ty::Tuple::Tuple(parameter_types));
        let parameters_signature = Signature::new(Some(parameters_tuple));
        MetaMethod {
            uid: self.uid,
            return_signature: self.return_value.signature,
            name: self.name,
            parameters_signature,
            description: self.description,
            parameters,
            return_description: self.return_value.description,
        }
    }
}

#[derive(Default, Debug)]
pub struct MetaMethodBuilderReturnValue {
    signature: Signature,
    description: String,
}

impl MetaMethodBuilderReturnValue {
    pub fn set_description<T: Into<String>>(&mut self, description: T) -> &mut Self {
        self.description = description.into();
        self
    }

    pub fn set_type<T: Into<Option<Type>>>(&mut self, ty: T) -> &mut Self {
        self.set_signature(Signature::new(ty.into()))
    }

    pub fn set_signature<T: Into<Signature>>(&mut self, signature: T) -> &mut Self {
        self.signature = signature.into();
        self
    }
}

#[derive(Default, Debug)]
pub struct MetaMethodBuilderParameter {
    parameter: MetaMethodParameter,
    ty: Option<Type>,
}

impl MetaMethodBuilderParameter {
    pub fn set_name<T: Into<String>>(&mut self, name: T) -> &mut Self {
        self.parameter.name = name.into();
        self
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) -> &mut Self {
        self.parameter.description = description.into();
        self
    }

    pub fn set_type<T: Into<Option<Type>>>(&mut self, ty: T) -> &mut Self {
        self.ty = ty.into();
        self
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
    qi_macros::Valuable,
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
    qi_macros::Valuable,
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
    qi_macros::Valuable,
    serde::Serialize,
    serde::Deserialize,
)]
#[qi(value = "crate")]
pub struct MetaProperty {
    pub uid: ActionId,
    pub name: String,
    pub signature: Signature,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum MemberAddress {
    Id(ActionId),
    Name(String),
}

impl From<ActionId> for MemberAddress {
    fn from(value: ActionId) -> Self {
        Self::Id(value)
    }
}

impl From<String> for MemberAddress {
    fn from(value: String) -> Self {
        Self::Name(value)
    }
}

impl From<&str> for MemberAddress {
    fn from(value: &str) -> Self {
        Self::Name(value.to_owned())
    }
}

impl PartialEq<&str> for MemberAddress {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Self::Name(name) => name == other,
            _ => false,
        }
    }
}

impl PartialEq<String> for MemberAddress {
    fn eq(&self, other: &String) -> bool {
        match self {
            Self::Name(name) => name == other,
            _ => false,
        }
    }
}

impl PartialEq<ActionId> for MemberAddress {
    fn eq(&self, other: &ActionId) -> bool {
        match self {
            Self::Id(id) => id == other,
            _ => false,
        }
    }
}

impl std::fmt::Display for MemberAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberAddress::Id(id) => id.fmt(f),
            MemberAddress::Name(name) => name.fmt(f),
        }
    }
}

impl<'a> IntoValue<'a> for MemberAddress {
    fn into_value(self) -> Value<'a> {
        match self {
            MemberAddress::Id(id) => id.into_value(),
            MemberAddress::Name(name) => name.into_value(),
        }
    }
}

impl<'a> FromValue<'a> for MemberAddress {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        // IMPROVE: not ideal to clone the value here.
        if let Ok(id) = ActionId::from_value(value.clone()) {
            Ok(Self::Id(id))
        } else if let Ok(name) = String::from_value(value.clone()) {
            Ok(Self::Name(name))
        } else {
            Err(FromValueError::TypeMismatch {
                expected: "an object member address".to_owned(),
                actual: value.to_string(),
            })
        }
    }
}
