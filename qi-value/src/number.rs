use derive_more::{From, TryInto};

// Serialize is derived, but Deserialize is not, because of its behavior for untagged enums:
//   "Serde will try to match the data against each variant in order and the first one that
//   deserializes successfully is the one returned."
#[derive(
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    From,
    TryInto,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
)]
pub enum Number {
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
}

impl Default for Number {
    fn default() -> Self {
        Self::Int32(0)
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
            Self::Float32(f) => visitor.visit_f32(f),
            Self::Float64(d) => visitor.visit_f64(d),
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
    use assert_matches::assert_matches;

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
}
