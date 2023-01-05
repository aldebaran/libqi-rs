use crate::{Error, Type};
use derive_more::{From, TryInto};
use ordered_float::OrderedFloat;

pub type Bool = bool;
pub type Int8 = i8;
pub type UInt8 = u8;
pub type Int16 = i16;
pub type UInt16 = u16;
pub type Int32 = i32;
pub type UInt32 = u32;
pub type Int64 = i64;
pub type UInt64 = u64;
pub type Float32 = OrderedFloat<f32>;
pub type Float64 = OrderedFloat<f64>;

pub(crate) const FALSE_BOOL: u8 = 0;
pub(crate) const TRUE_BOOL: u8 = 1;

// Serialize is derived, but Deserialize is not, because of its behavior for untagged enums:
//   "Serde will try to match the data against each variant in order and the first one that
//   deserializes successfully is the one returned."
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, TryInto, Hash, Debug, serde::Serialize,
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
        match self {
            Self::Int8(_) => Type::Int8,
            Self::UInt8(_) => Type::UInt8,
            Self::Int16(_) => Type::Int16,
            Self::UInt16(_) => Type::UInt16,
            Self::Int32(_) => Type::Int32,
            Self::UInt32(_) => Type::UInt32,
            Self::Int64(_) => Type::Int64,
            Self::UInt64(_) => Type::UInt64,
            Self::Float32(_) => Type::Float32,
            Self::Float64(_) => Type::Float64,
        }
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

impl<'de> serde::Deserialize<'de> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Number;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number")
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Number::from(v))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl<'de> serde::de::Deserializer<'de> for Number {
    type Error = Error;

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

impl<'de> serde::de::IntoDeserializer<'de, Error> for Number {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_number_from_f32() {
        assert_eq!(Number::from(1f32), Number::Float32(OrderedFloat(1.)));
    }

    #[test]
    fn test_number_from_f64() {
        assert_eq!(Number::from(1f64), Number::Float64(OrderedFloat(1.)));
    }

    #[test]
    fn test_number_deserializer() {
        use serde::de::{Deserialize, IntoDeserializer};
        use serde_value::Value;
        let value_deserialize = |n: Number| Value::deserialize(n.into_deserializer());
        assert_matches!(value_deserialize(Number::from(1i8)), Ok(Value::I8(1)));
        assert_matches!(value_deserialize(Number::from(1u8)), Ok(Value::U8(1)));
        assert_matches!(value_deserialize(Number::from(1i16)), Ok(Value::I16(1)));
        assert_matches!(value_deserialize(Number::from(1u16)), Ok(Value::U16(1)));
        assert_matches!(value_deserialize(Number::from(1i32)), Ok(Value::I32(1)));
        assert_matches!(value_deserialize(Number::from(1u32)), Ok(Value::U32(1)));
        assert_matches!(value_deserialize(Number::from(1i64)), Ok(Value::I64(1)));
        assert_matches!(value_deserialize(Number::from(1u64)), Ok(Value::U64(1)));
        assert_matches!(value_deserialize(Number::from(1f32)), Ok(Value::F32(f)) => assert_eq!(f, 1.));
        assert_matches!(value_deserialize(Number::from(1f64)), Ok(Value::F64(f)) => assert_eq!(f, 1.));
    }

    #[test]
    fn test_number_get_type() {
        assert_eq!(Number::from(1i8).get_type(), Type::Int8);
        assert_eq!(Number::from(1u8).get_type(), Type::UInt8);
        assert_eq!(Number::from(1i16).get_type(), Type::Int16);
        assert_eq!(Number::from(1u16).get_type(), Type::UInt16);
        assert_eq!(Number::from(1i32).get_type(), Type::Int32);
        assert_eq!(Number::from(1u32).get_type(), Type::UInt32);
        assert_eq!(Number::from(1i64).get_type(), Type::Int64);
        assert_eq!(Number::from(1u64).get_type(), Type::UInt64);
        assert_eq!(Number::from(1f32).get_type(), Type::Float32);
        assert_eq!(Number::from(1f64).get_type(), Type::Float64);
    }

    #[test]
    fn test_number_serde() {
        assert_tokens(&Number::from(1i8), &[Token::I8(1)]);
        assert_tokens(&Number::from(1u8), &[Token::U8(1)]);
        assert_tokens(&Number::from(1i16), &[Token::I16(1)]);
        assert_tokens(&Number::from(1u16), &[Token::U16(1)]);
        assert_tokens(&Number::from(1i32), &[Token::I32(1)]);
        assert_tokens(&Number::from(1u32), &[Token::U32(1)]);
        assert_tokens(&Number::from(1i64), &[Token::I64(1)]);
        assert_tokens(&Number::from(1u64), &[Token::U64(1)]);
        assert_tokens(&Number::from(1f32), &[Token::F32(1.)]);
        assert_tokens(&Number::from(1f64), &[Token::F64(1.)]);
    }
}
