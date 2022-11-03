pub use super::ser::Error;
use super::{tuple, AnyValue};
use serde::{
    de::{value::MapDeserializer, IntoDeserializer},
    forward_to_deserialize_any,
};

pub fn from_any_value<T>(d: &AnyValue) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(d)
}

impl<'de> serde::Deserializer<'de> for AnyValue {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            AnyValue::Void => visitor.visit_unit(),
            AnyValue::Bool(b) => visitor.visit_bool(b),
            AnyValue::Int8(i) => visitor.visit_i8(i),
            AnyValue::UInt8(i) => visitor.visit_u8(i),
            AnyValue::Int16(i) => visitor.visit_i16(i),
            AnyValue::UInt16(i) => visitor.visit_u16(i),
            AnyValue::Int32(i) => visitor.visit_i32(i),
            AnyValue::UInt32(i) => visitor.visit_u32(i),
            AnyValue::Int64(i) => visitor.visit_i64(i),
            AnyValue::UInt64(i) => visitor.visit_u64(i),
            AnyValue::Float(f) => visitor.visit_f32(f),
            AnyValue::Double(d) => visitor.visit_f64(d),
            AnyValue::String(s) => visitor.visit_string(s),
            AnyValue::Raw(buf) => visitor.visit_byte_buf(buf),
            AnyValue::Option { option, .. } => match option {
                Some(v) => visitor.visit_some(v.as_ref()),
                None => visitor.visit_none(),
            },
            AnyValue::List { list, .. } => visitor.visit_seq(list.into_deserializer()),
            AnyValue::Map { map, .. } => visitor.visit_map(MapDeserializer::new(map.into_iter())),
            AnyValue::Tuple(_) => todo!(),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!("enums are not yet supported as an AnyValue")
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char str string bytes byte_buf option unit
        tuple unit_struct tuple_struct struct newtype_struct
        seq map identifier ignored_any
    }
}

impl<'de> serde::Deserializer<'de> for &AnyValue {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            AnyValue::Void => visitor.visit_unit(),
            AnyValue::Bool(b) => visitor.visit_bool(*b),
            AnyValue::Int8(i) => visitor.visit_i8(*i),
            AnyValue::UInt8(i) => visitor.visit_u8(*i),
            AnyValue::Int16(i) => visitor.visit_i16(*i),
            AnyValue::UInt16(i) => visitor.visit_u16(*i),
            AnyValue::Int32(i) => visitor.visit_i32(*i),
            AnyValue::UInt32(i) => visitor.visit_u32(*i),
            AnyValue::Int64(i) => visitor.visit_i64(*i),
            AnyValue::UInt64(i) => visitor.visit_u64(*i),
            AnyValue::Float(f) => visitor.visit_f32(*f),
            AnyValue::Double(d) => visitor.visit_f64(*d),
            AnyValue::String(s) => visitor.visit_string(*s),
            AnyValue::Raw(buf) => visitor.visit_byte_buf(*buf),
            AnyValue::Option { option, .. } => match option {
                Some(v) => visitor.visit_some(v.as_ref()),
                None => visitor.visit_none(),
            },
            AnyValue::List { list, .. } => visitor.visit_seq(list.into_deserializer()),
            AnyValue::Map { map, .. } => visitor.visit_map(MapDeserializer::new(map.into_iter())),
            AnyValue::Tuple(_) => todo!(),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!("enums are not yet supported as an AnyValue")
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char str string bytes byte_buf option unit
        tuple unit_struct tuple_struct struct newtype_struct
        seq map identifier ignored_any
    }
}

impl<'de> serde::Deserialize<'de> for AnyValue {
    fn deserialize<D>(deserializer: D) -> Result<AnyValue, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        //struct Visitor;
        //impl<'de> serde::de::Visitor<'de> for Visitor {
        //    type Value = AnyValue;

        //    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        //        formatter.write_str("an \"any value\"")
        //    }

        //    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Bool(value))
        //    }

        //    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Int8(value))
        //    }

        //    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E> {
        //        Ok(AnyValue::UInt8(value))
        //    }

        //    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Int16(value))
        //    }

        //    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E> {
        //        Ok(AnyValue::UInt16(value))
        //    }

        //    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Int32(value))
        //    }

        //    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E> {
        //        Ok(AnyValue::UInt32(value))
        //    }

        //    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Int64(value))
        //    }

        //    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        //        Ok(AnyValue::UInt64(value))
        //    }

        //    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Float(value))
        //    }

        //    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Double(value))
        //    }

        //    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        //    where
        //        E: serde::de::Error,
        //    {
        //        self.visit_string(value.to_string())
        //    }

        //    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        //        Ok(AnyValue::String(value))
        //    }

        //    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        //    where
        //        E: serde::de::Error,
        //    {
        //        self.visit_byte_buf(v.to_vec())
        //    }

        //    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Raw(value))
        //    }

        //    fn visit_none<E>(self) -> Result<Self::Value, E> {
        //        Ok(AnyValue::Option(None))
        //    }

        //    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        //    where
        //        D: serde::Deserializer<'de>,
        //    {
        //        let v = serde::Deserialize::deserialize(deserializer)?;
        //        Ok(AnyValue::Option(Some(Box::new(v))))
        //    }

        //    fn visit_unit<E>(self) -> Result<Value, E> {
        //        Ok(AnyValue::Void)
        //    }

        //    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        //    where
        //        D: serde::Deserializer<'de>,
        //    {
        //        Ok(AnyValue::Tuple(serde::Deserialize::deserialize(
        //            deserializer,
        //        )?))
        //    }

        //    fn visit_seq<V>(self, mut seq: V) -> Result<Value, V::Error>
        //    where
        //        V: serde::de::SeqAccess<'de>,
        //    {
        //        let mut vec = Vec::new();
        //        while let Some(elem) = seq.next_element()? {
        //            vec.push(elem);
        //        }
        //        Ok(AnyValue::List(vec))
        //    }

        //    fn visit_map<V>(self, mut map: V) -> Result<Value, V::Error>
        //    where
        //        V: serde::de::MapAccess<'de>,
        //    {
        //        let mut vec = Vec::new();
        //        while let Some(pair) = map.next_entry()? {
        //            vec.push(pair);
        //        }
        //        Ok(AnyValue::Map(vec))
        //    }

        //    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        //    where
        //        A: serde::de::EnumAccess<'de>,
        //    {
        //        // TODO ?
        //        let _ = data;
        //        Err(serde::de::Error::invalid_type(
        //            serde::de::Unexpected::Enum,
        //            &self,
        //        ))
        //    }
        //}

        use crate::typesystem::{Signature, Type};
        use std::str::FromStr;
        enum Visitor {
            Signature,
            AnyValue(Type),
        }
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = AnyValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an \"any value\" as a tuple of a signature and a value")
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let signature: Signature = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let value_type = signature.into_type();
                match value_type {
                    Type::None => todo!(),
                    Type::Unknown => todo!(),
                    Type::Void => todo!(),
                    Type::Bool => todo!(),
                    Type::Int8 => todo!(),
                    Type::UInt8 => todo!(),
                    Type::Int16 => todo!(),
                    Type::UInt16 => todo!(),
                    Type::Int32 => todo!(),
                    Type::UInt32 => todo!(),
                    Type::Int64 => todo!(),
                    Type::UInt64 => todo!(),
                    Type::Float => todo!(),
                    Type::Double => todo!(),
                    Type::String => todo!(),
                    Type::Raw => todo!(),
                    Type::Object => todo!(),
                    Type::Dynamic => todo!(),
                    Type::Option(_) => todo!(),
                    Type::List(_) => todo!(),
                    Type::Map { key, value } => todo!(),
                    Type::Tuple(_) => todo!(),
                    Type::VarArgs(_) => todo!(),
                    Type::KwArgs(_) => todo!(),
                }
            }
        }

        deserializer.deserialize_tuple(2, Visitor::Signature)
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for AnyValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for &AnyValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}
