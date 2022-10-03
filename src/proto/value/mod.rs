mod de;
mod ser;

use super::Object;
use std::collections::HashMap;
use thiserror::Error;

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

struct TupleMember {
    name: Option<String>,
    value: Value,
}

pub enum Value {
    Void,
    Bool(bool),
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Float(f32),
    Double(f64),
    String(String),
    List(Vec<Value>),
    Map(HashMap<Value, Value>),
    Object(Box<dyn Object>),
    Tuple {
        name: Option<String>,
        members: Vec<TupleMember>,
    },
    Raw(Vec<u8>),
    Optional(Box<Value>),
}

fn to_value<T>(value: &T) -> Result<Value>
where
    T: serde::Serialize + ?Sized,
{
    let ser = ser::Serializer {};
    value.serialize(ser)
}

fn from_value<T>(value: Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let deser = de::Deserializer::new(value);
    T::deserialize(deser)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("error: {0}")]
    Custom(String),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_value() {
        todo!()
    }

    #[test]
    fn from_value() {
        todo!()
    }
}
