mod de;
mod ser;
pub mod tuple;
pub use tuple::Tuple;

//pub enum Type {
//    Void,
//    Bool,
//    Int8,
//    UInt8,
//    Int16,
//    UInt16,
//    Int32,
//    UInt32,
//    Float,
//    Double,
//    String,
//    List(Box<Type>),
//    Map { key: Box<Type>, value: Box<Type> },
//    Object,
//    Tuple(Vec<Type>),
//    Raw,
//    VarArgs(Box<Type>),
//    KwArgs(Box<Type>),
//    Optional(Box<Type>),
//    Dynamic,
//    Unknown,
//}

// TODO: #[non_exhaustive]
#[derive(Default, Clone, PartialEq, PartialOrd, Debug)]
pub enum Value {
    #[default]
    Void,
    Bool(bool),
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float(f32),
    Double(f64),
    String(String),
    List(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Tuple(Tuple),
    Raw(Vec<u8>),
    Optional(Option<Box<Value>>),
}

impl Value {
    pub fn as_tuple(&self) -> Option<&Tuple> {
        if let Self::Tuple(tuple) = self {
            Some(tuple)
        } else {
            None
        }
    }

    pub fn as_tuple_mut(&mut self) -> Option<&mut Tuple> {
        if let Self::Tuple(tuple) = self {
            Some(tuple)
        } else {
            None
        }
    }
}

impl std::str::FromStr for Value {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Value::String(s.to_string()))
    }
}

// TODO: Implement all conversions

impl From<Tuple> for Value {
    fn from(t: Tuple) -> Self {
        Value::Tuple(t)
    }
}

impl TryFrom<Value> for Tuple {
    type Error = TryFromValueError;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::Tuple(t) => Ok(t),
            _ => Err(TryFromValueError),
        }
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: serde::Serialize + ?Sized,
{
    value.serialize(ser::Serializer)
}

pub fn from_value<T>(value: Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(value)
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Error {
    #[error("error: {0}")]
    Custom(String),

    #[error("union types are not supported in the qi type system")]
    UnionAreNotSupported,

    #[error("a map key is missing")]
    MissingMapKey,

    #[error("value cannot be deserialized")]
    ValueCannotBeDeserialized,
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[error("value conversion failed")]
pub struct TryFromValueError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::tests::Serializable;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_value_from_string() {
        use std::str::FromStr;
        let value = Value::from_str("cookies recipe").unwrap();
        assert_eq!(value, Value::String("cookies recipe".to_string()));
    }

    #[test]
    fn test_value_as_tuple() {
        assert_eq!(
            Value::Tuple(Default::default()).as_tuple(),
            Some(&Tuple::default())
        );
        assert_eq!(Value::Int32(42).as_tuple(), None);
    }

    #[test]
    fn test_value_as_tuple_mut() {
        assert_eq!(
            Value::Tuple(Default::default()).as_tuple_mut(),
            Some(&mut Tuple::default())
        );
        assert_eq!(Value::Int32(42).as_tuple_mut(), None);
    }

    #[test]
    fn test_to_value() {
        let (s, expected) = Serializable::sample_and_value();
        let value = to_value(&s).expect("serialization error");
        assert_eq!(value, expected);
    }

    #[test]
    fn test_from_value() {
        let (expected, v) = Serializable::sample_and_value();
        let s: Serializable = from_value(v).expect("deserialization error");
        assert_eq!(s, expected);
    }

    #[test]
    fn test_to_from_value_invariant() -> Result<()> {
        let (s, _) = Serializable::sample_and_value();
        let s2: Serializable = from_value(to_value(&s)?)?;
        assert_eq!(s, s2);
        Ok(())
    }
}
