pub mod de;
mod impls;
mod ser;

use crate::{map::Map, ty::DisplayTypeOption, Object, Reflect, Signature, Type};
use ordered_float::OrderedFloat;
use std::borrow::Cow;

/// The [`Value`] structure represents any value of the `qi` type system.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Value<'a> {
    Unit,
    Bool(bool),
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float32(OrderedFloat<f32>),
    Float64(OrderedFloat<f64>),
    String(Cow<'a, str>),
    Raw(Cow<'a, [u8]>),
    Option(Option<Box<Value<'a>>>),
    List(Vec<Value<'a>>),
    Map(Map<Value<'a>, Value<'a>>),
    Tuple(Vec<Value<'a>>),
    Object(Box<Object>),
    Dynamic(Box<Value<'a>>),
}

impl<'a> Value<'a> {
    pub fn cast<T>(self) -> Result<T, FromValueError>
    where
        T: FromValue<'a>,
    {
        T::from_value(self)
    }
}

impl Default for Value<'_> {
    fn default() -> Self {
        Self::Unit
    }
}

impl std::fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Unit => f.write_str("unit"),
            Value::Bool(v) => write!(f, "bool({})", v),
            Value::Int8(v) => write!(f, "i8({})", v),
            Value::UInt8(v) => write!(f, "u8({})", v),
            Value::Int16(v) => write!(f, "i16({})", v),
            Value::UInt16(v) => write!(f, "u16({})", v),
            Value::Int32(v) => write!(f, "i32({})", v),
            Value::UInt32(v) => write!(f, "u32({})", v),
            Value::Int64(v) => write!(f, "i64({})", v),
            Value::UInt64(v) => write!(f, "u64({})", v),
            Value::Float32(v) => write!(f, "f32({})", v),
            Value::Float64(v) => write!(f, "f64({})", v),
            Value::String(v) => write!(f, "str({})", v),
            Value::Raw(v) => write!(f, "raw(len={})", v.len()),
            Value::Option(v) => {
                f.write_str("opt(")?;
                match v {
                    Some(v) => v.fmt(f)?,
                    None => f.write_str("none")?,
                };
                f.write_str(")")
            }
            Value::List(l) => {
                write!(f, "list[{}](", l.len())?;
                let mut add_sep = false;
                for v in l {
                    if add_sep {
                        f.write_str(",")?;
                    }
                    v.fmt(f)?;
                    add_sep = true;
                }
                f.write_str(")")
            }
            Value::Map(m) => {
                write!(f, "map[{}](", m.len())?;
                let mut add_sep = false;
                for (k, v) in m {
                    if add_sep {
                        f.write_str(",")?;
                    }
                    write!(f, "{k}:{v}")?;
                    add_sep = true;
                }
                f.write_str(")")
            }
            Value::Tuple(elems) => {
                write!(f, "tuple(")?;
                let mut add_sep = false;
                for v in elems {
                    if add_sep {
                        f.write_str(",")?;
                    }
                    v.fmt(f)?;
                    add_sep = true;
                }
                f.write_str(")")
            }
            Value::Object(object) => object.fmt(f),
            Value::Dynamic(v) => v.fmt(f),
        }
    }
}

pub trait AsValue {
    fn value_type(&self) -> Type;

    fn value_signature(&self) -> Signature {
        Signature(Some(self.value_type()))
    }

    fn as_value(&self) -> Value<'_>;
}

pub trait FromValue<'a>: Sized {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError>;
}

#[derive(thiserror::Error, Debug)]
pub enum FromValueError {
    TypeMismatch { expected: String, actual: String },
    Custom(String),
}

impl std::fmt::Display for FromValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FromValueError::TypeMismatch { expected, actual } => {
                write!(
                    f,
                    "value type mismatch: expected {expected}, but value is {actual}",
                )
            }
            FromValueError::Custom(s) => f.write_str(s),
        }
    }
}

impl FromValueError {
    pub fn value_type_mismatch<Dst>(value: &impl AsValue) -> Self
    where
        Dst: Reflect,
    {
        Self::TypeMismatch {
            expected: DisplayTypeOption(&Dst::ty()).to_string(),
            actual: value.value_type().to_string(),
        }
    }
}

impl From<std::convert::Infallible> for FromValueError {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}
