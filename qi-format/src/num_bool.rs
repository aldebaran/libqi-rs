use crate::Type;
use derive_more::{From, TryInto};
use ordered_float::OrderedFloat;

/// # De/Serialization
///
/// This is represented as a `bool` in the Serde data model.
pub type Bool = bool;

/// # De/Serialization
///
/// This is represented as a `i8` in the Serde data model.
pub type Int8 = i8;

/// # De/Serialization
///
/// This is represented as a `u8` in the Serde data model.
pub type UInt8 = u8;

/// # De/Serialization
///
/// This is represented as a `i16` in the Serde data model.
pub type Int16 = i16;

/// # De/Serialization
///
/// This is represented as a `u16` in the Serde data model.
pub type UInt16 = u16;

/// # De/Serialization
///
/// This is represented as a `i32` in the Serde data model.
pub type Int32 = i32;

/// # De/Serialization
///
/// This is represented as a `u32` in the Serde data model.
pub type UInt32 = u32;

/// # De/Serialization
///
/// This is represented as a `i64` in the Serde data model.
pub type Int64 = i64;

/// # De/Serialization
///
/// This is represented as a `u64` in the Serde data model.
pub type UInt64 = u64;

/// # De/Serialization
///
/// This is represented as a `f32` in the Serde data model.
pub type Float32 = OrderedFloat<f32>;

/// # De/Serialization
///
/// This is represented as a `f64` in the Serde data model.
pub type Float64 = OrderedFloat<f64>;

pub(crate) const FALSE_BOOL: u8 = 0;
pub(crate) const TRUE_BOOL: u8 = 1;

/// # De/Serialization
///
/// This is represented transparently with its content.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    From,
    TryInto,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(untagged)]
pub enum Number {
    Int8(Int8),
    UInt8(UInt8),
    Int16(Int16),
    UInt16(UInt16),
    Int32(Int32),
    UInt32(UInt32),
    Int64(Int64),
    UInt64(UInt64),
    Float32(Float32),
    Float64(Float64),
}

impl Number {
    pub fn get_type(&self) -> Type {
        todo!()
        //Value::Int8(_) => Type::Int8,
        //Value::UInt8(_) => Type::UInt8,
        //Value::Int16(_) => Type::UInt16,
        //Value::UInt16(_) => Type::UInt16,
        //Value::Int32(_) => Type::Int32,
        //Value::UInt32(_) => Type::UInt32,
        //Value::Int64(_) => Type::UInt32,
        //Value::UInt64(_) => Type::UInt64,
        //Value::Float32(_) => Type::Float,
        //Value::Float64(_) => Type::Double,
    }
}

impl From<f32> for Number {
    fn from(f: f32) -> Self {
        Number::from(OrderedFloat(f))
    }
}

impl From<f64> for Number {
    fn from(f: f64) -> Self {
        Number::from(OrderedFloat(f))
    }
}

impl<'de> serde::de::Deserializer<'de> for Number {
    type Error = serde::de::value::Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char str string bytes byte_buf option unit
        tuple unit_struct tuple_struct struct newtype_struct
        seq map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Self::Int8(i) => visitor.visit_i8(i),
            Self::UInt8(i) => visitor.visit_u8(i),
            Self::Int16(i) => visitor.visit_i16(i),
            Self::UInt16(i) => visitor.visit_u16(i),
            Self::Int32(i) => visitor.visit_i32(i),
            Self::UInt32(i) => visitor.visit_u32(i),
            Self::Int64(i) => visitor.visit_i64(i),
            Self::UInt64(i) => visitor.visit_u64(i),
            Self::Float32(f) => visitor.visit_f32(*f),
            Self::Float64(d) => visitor.visit_f64(*d),
        }
    }
}

impl<'de> serde::de::IntoDeserializer<'de, serde::de::value::Error> for Number {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_bool_serde() {
        assert_tokens(&Bool::from(true), &[Token::Bool(true)]);
        assert_tokens(&Bool::from(false), &[Token::Bool(false)]);
    }

    #[test]
    fn test_int8_serde() {
        assert_tokens(&Int8::from(12), &[Token::I8(12)]);
    }

    #[test]
    fn test_uint8_serde() {
        assert_tokens(&UInt8::from(32), &[Token::U8(32)]);
    }

    #[test]
    fn test_int16_serde() {
        assert_tokens(&Int16::from(-2310i16), &[Token::I16(-2310)]);
    }

    #[test]
    fn test_uint16_serde() {
        assert_tokens(&UInt16::from(3920u16), &[Token::U16(3920)]);
    }

    #[test]
    fn test_int32_serde() {
        assert_tokens(&Int32::from(-321), &[Token::I32(-321)]);
    }

    #[test]
    fn test_uint32_serde() {
        assert_tokens(&UInt32::from(23901u32), &[Token::U32(23901u32)]);
    }

    #[test]
    fn test_int64_serde() {
        assert_tokens(&Int64::from(-23901), &[Token::I64(-23901)]);
    }

    #[test]
    fn test_uint64_serde() {
        assert_tokens(&UInt64::from(23901u64), &[Token::U64(23901)]);
    }

    #[test]
    fn test_f32_serde() {
        assert_tokens(&Float32::from(1.), &[Token::F32(1.)]);
    }

    #[test]
    fn test_f64_serde() {
        assert_tokens(&Float64::from(1.), &[Token::F64(1.)]);
    }

    #[test]
    fn test_number_get_type() {
        assert_eq!(Number::Int8(1).get_type(), Type::UInt8);
        assert_eq!(Number::UInt8(1).get_type(), Type::UInt8);
        assert_eq!(Number::Int16(1).get_type(), Type::UInt16);
        assert_eq!(Number::UInt16(1).get_type(), Type::UInt16);
        assert_eq!(Number::Int32(1).get_type(), Type::UInt32);
        assert_eq!(Number::UInt32(1).get_type(), Type::UInt32);
        assert_eq!(Number::Int64(1).get_type(), Type::UInt64);
        assert_eq!(Number::UInt64(1).get_type(), Type::UInt64);
        assert_eq!(Number::from(Float32::from(1.)).get_type(), Type::Float);
        assert_eq!(Number::from(Float64::from(1.)).get_type(), Type::Double);
    }

    #[test]
    fn test_number_serde() {
        assert_tokens(&Number::from(Int32::from(219)), &[Token::I32(219)]);
        assert_tokens(&Number::from(UInt64::from(92180u64)), &[Token::U64(92180)]);
    }
}
