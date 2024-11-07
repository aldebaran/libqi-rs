mod de;

use serde::Serialize;

use crate::{reflect::RuntimeReflect, FromValue, IntoValue, Reflect, ToValue, Value};

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::From)]
pub struct Dynamic<T>(pub T);

impl<'a> Dynamic<Value<'a>> {
    pub fn into_owned(self) -> Dynamic<Value<'static>> {
        Dynamic(self.0.into_owned())
    }
}

impl<T> std::fmt::Display for Dynamic<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Reflect for Dynamic<T> {
    fn ty() -> Option<crate::Type> {
        None
    }
}

impl<T> ToValue for Dynamic<T>
where
    T: ToValue,
{
    fn to_value(&self) -> Value<'_> {
        Value::Dynamic(Box::new(self.0.to_value()))
    }
}

impl<'a, T> IntoValue<'a> for Dynamic<T>
where
    T: IntoValue<'a>,
{
    fn into_value(self) -> Value<'a> {
        Value::Dynamic(Box::new(self.0.into_value()))
    }
}

impl<'a, T> FromValue<'a> for Dynamic<T>
where
    T: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, crate::FromValueError> {
        match value {
            Value::Dynamic(v) => Ok(Self(T::from_value(*v)?)),
            _ => Err(crate::FromValueError::TypeMismatch {
                expected: "a dynamic value".to_owned(),
                actual: value.to_string(),
            }),
        }
    }
}

impl<T> serde::Serialize for Dynamic<T>
where
    for<'a> &'a T: IntoValue<'a>,
    T: RuntimeReflect,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self::serialize(&self.0, serializer)
    }
}

impl<'de, T> serde::Deserialize<'de> for Dynamic<T>
where
    T: FromValue<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self::deserialize(deserializer).map(Self)
    }
}

impl<T, U> serde_with::SerializeAs<Dynamic<T>> for Dynamic<U>
where
    U: serde_with::SerializeAs<T>,
{
    fn serialize_as<S>(source: &Dynamic<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_with::ser::SerializeAsWrap::<T, U>::new(&source.0).serialize(serializer)
    }
}

impl<'de, T, U> serde_with::DeserializeAs<'de, Dynamic<T>> for Dynamic<U>
where
    U: serde_with::DeserializeAs<'de, T>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Dynamic<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        Ok(Dynamic(
            serde_with::de::DeserializeAsWrap::<T, U>::deserialize(deserializer)?.into_inner(),
        ))
    }
}

const SERDE_STRUCT_NAME: &str = "Dynamic";

enum Fields {
    Signature,
    Value,
}

impl Fields {
    const KEYS: [&'static str; 2] = ["signature", "value"];
    const fn key(&self) -> &'static str {
        match self {
            Fields::Signature => Self::KEYS[0],
            Fields::Value => Self::KEYS[1],
        }
    }
}

pub fn serialize<'a, T, S>(value: T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: IntoValue<'a> + RuntimeReflect,
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;
    let mut serializer = serializer.serialize_struct(SERDE_STRUCT_NAME, Fields::KEYS.len())?;
    serializer.serialize_field(Fields::Signature.key(), &value.signature())?;
    serializer.serialize_field(Fields::Value.key(), &value.into_value())?;
    serializer.end()
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromValue<'de>,
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value =
        deserializer.deserialize_struct(SERDE_STRUCT_NAME, &Fields::KEYS, de::DynamicVisitor)?;
    value
        .cast_into()
        .map_err(|err| D::Error::custom(err.to_string()))
}
