mod de;
mod ser;

use super::Object;
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

#[derive(Debug)]
struct TupleMember {
    name: Option<String>,
    value: Value,
}

impl PartialEq for TupleMember {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
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
    Int64(i64),
    UInt64(u64),
    Float(f32),
    Double(f64),
    String(String),
    List(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Object(Box<dyn Object>),
    Tuple {
        name: Option<String>,
        members: Vec<TupleMember>,
    },
    Raw(Vec<u8>),
    Optional(Option<Box<Value>>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Int8(l0), Self::Int8(r0)) => l0 == r0,
            (Self::UInt8(l0), Self::UInt8(r0)) => l0 == r0,
            (Self::Int16(l0), Self::Int16(r0)) => l0 == r0,
            (Self::UInt16(l0), Self::UInt16(r0)) => l0 == r0,
            (Self::Int32(l0), Self::Int32(r0)) => l0 == r0,
            (Self::UInt32(l0), Self::UInt32(r0)) => l0 == r0,
            (Self::Int64(l0), Self::Int64(r0)) => l0 == r0,
            (Self::UInt64(l0), Self::UInt64(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Double(l0), Self::Double(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::List(l0), Self::List(r0)) => l0 == r0,
            (Self::Map(l0), Self::Map(r0)) => l0 == r0,
            // Objects are not comparable
            (Self::Object(l0), Self::Object(r0)) => false,
            (
                Self::Tuple {
                    name: l_name,
                    members: l_members,
                },
                Self::Tuple {
                    name: r_name,
                    members: r_members,
                },
            ) => l_name == r_name && l_members == r_members,
            (Self::Raw(l0), Self::Raw(r0)) => l0 == r0,
            (Self::Optional(l0), Self::Optional(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => write!(f, "Void"),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Int8(arg0) => f.debug_tuple("Int8").field(arg0).finish(),
            Self::UInt8(arg0) => f.debug_tuple("UInt8").field(arg0).finish(),
            Self::Int16(arg0) => f.debug_tuple("Int16").field(arg0).finish(),
            Self::UInt16(arg0) => f.debug_tuple("UInt16").field(arg0).finish(),
            Self::Int32(arg0) => f.debug_tuple("Int32").field(arg0).finish(),
            Self::UInt32(arg0) => f.debug_tuple("UInt32").field(arg0).finish(),
            Self::Int64(arg0) => f.debug_tuple("Int64").field(arg0).finish(),
            Self::UInt64(arg0) => f.debug_tuple("UInt64").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::Double(arg0) => f.debug_tuple("Double").field(arg0).finish(),
            Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Self::List(arg0) => f.debug_tuple("List").field(arg0).finish(),
            Self::Map(arg0) => f.debug_tuple("Map").field(arg0).finish(),
            Self::Object(arg0) => f.write_str("Object"),
            Self::Tuple { name, members } => f
                .debug_struct("Tuple")
                .field("name", name)
                .field("members", members)
                .finish(),
            Self::Raw(arg0) => f.debug_tuple("Raw").field(arg0).finish(),
            Self::Optional(arg0) => f.debug_tuple("Optional").field(arg0).finish(),
        }
    }
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

    #[error("union types are not supported in the qi type system")]
    UnionAreNotSupported,

    #[error("a map key is missing")]
    MissingMapKey,
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
