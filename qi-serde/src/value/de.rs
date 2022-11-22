use super::*;
use serde::{
    de::{value::MapDeserializer, value::SeqDeserializer, IntoDeserializer},
    forward_to_deserialize_any,
};

pub fn from_value<T>(d: Value) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(d)
}

pub fn from_borrowed_value<'v, T>(d: &'v Value) -> Result<T, Error>
where
    T: serde::Deserialize<'v>,
{
    T::deserialize(d)
}

impl<'de> serde::Deserializer<'de> for Value<'de> {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    forward_to_deserialize_any! {
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
            Value::Unit => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Int8(i) => visitor.visit_i8(i),
            Value::UnsignedInt8(i) => visitor.visit_u8(i),
            Value::Int16(i) => visitor.visit_i16(i),
            Value::UnsignedInt16(i) => visitor.visit_u16(i),
            Value::Int32(i) => visitor.visit_i32(i),
            Value::UnsignedInt32(i) => visitor.visit_u32(i),
            Value::Int64(i) => visitor.visit_i64(i),
            Value::UnsignedInt64(i) => visitor.visit_u64(i),
            Value::Float32(f) => visitor.visit_f32(f),
            Value::Float64(d) => visitor.visit_f64(d),
            Value::String(s) => match s {
                Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                Cow::Owned(s) => visitor.visit_string(s),
            },
            Value::Raw(buf) => match buf {
                Cow::Borrowed(bytes) => visitor.visit_borrowed_bytes(bytes),
                Cow::Owned(buf) => visitor.visit_byte_buf(buf),
            },
            Value::Option(option) => match option {
                Some(v) => visitor.visit_some(v.into_deserializer()),
                None => visitor.visit_none(),
            },
            Value::List(elements) | Value::Tuple(Tuple { elements }) => {
                visitor.visit_seq(SeqDeserializer::new(elements.into_iter()))
            }
            Value::Map(map) => visitor.visit_map(MapDeserializer::new(map.into_iter())),
        }
    }
}

impl<'de> serde::Deserializer<'de> for &'de Value<'de> {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    forward_to_deserialize_any! {
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
            Value::Unit => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Int8(i) => visitor.visit_i8(*i),
            Value::UnsignedInt8(i) => visitor.visit_u8(*i),
            Value::Int16(i) => visitor.visit_i16(*i),
            Value::UnsignedInt16(i) => visitor.visit_u16(*i),
            Value::Int32(i) => visitor.visit_i32(*i),
            Value::UnsignedInt32(i) => visitor.visit_u32(*i),
            Value::Int64(i) => visitor.visit_i64(*i),
            Value::UnsignedInt64(i) => visitor.visit_u64(*i),
            Value::Float32(f) => visitor.visit_f32(*f),
            Value::Float64(d) => visitor.visit_f64(*d),
            Value::String(s) => visitor.visit_borrowed_str(s),
            Value::Raw(buf) => visitor.visit_borrowed_bytes(buf),
            Value::Option(option) => match option {
                Some(v) => visitor.visit_some(v.as_ref()),
                None => visitor.visit_none(),
            },
            Value::List(elements) | Value::Tuple(Tuple { elements }) => {
                visitor.visit_seq(SeqDeserializer::new(elements.iter()))
            }
            Value::Map(Map(map)) => {
                visitor.visit_map(MapDeserializer::new(map.iter().map(|(k, v)| (k, v))))
            }
        }
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for Value<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for &'de Value<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
#[error("{0}")]
pub struct Error(StdString);

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

impl<'de> Deserialize<'de> for Value<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Value<'de>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Bool(v))
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int8(v))
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::UnsignedInt8(v))
            }

            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int16(v))
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::UnsignedInt16(v))
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int32(v))
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::UnsignedInt32(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int64(v))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::UnsignedInt64(v))
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Float32(v))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Float64(v))
            }

            fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(v.to_string().into()))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(v.to_owned().into()))
            }

            fn visit_string<E>(self, v: StdString) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(v.into()))
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(v.into()))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Raw(v.to_owned().into()))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Raw(v.into()))
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Raw(v.into()))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Option(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = Value::deserialize(deserializer)?;
                Ok(Value::Option(Some(Box::new(value))))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut list = match seq.size_hint() {
                    Some(size) => List::with_capacity(size),
                    None => List::new(),
                };
                while let Some(element) = seq.next_element()? {
                    list.push(element);
                }
                Ok(Value::List(list))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut map_vec = match map.size_hint() {
                    Some(size) => Vec::with_capacity(size),
                    None => Vec::new(),
                };
                while let Some((key, value)) = map.next_entry()? {
                    map_vec.push((key, value));
                }
                Ok(Value::Map(Map::new(map_vec)))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Unit)
            }

            fn visit_enum<A>(self, _data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::EnumAccess<'de>,
            {
                todo!("enums are not yet supported as values")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = Value::deserialize(deserializer)?;
                Ok(Value::Tuple(Tuple::new(vec![value])))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl<'de> Deserialize<'de> for Map<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Map<'de>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut values = match map.size_hint() {
                    Some(size) => Vec::with_capacity(size),
                    None => Vec::new(),
                };
                while let Some((key, value)) = map.next_entry()? {
                    values.push((key, value))
                }
                Ok(Map(values))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

impl<'de> Deserialize<'de> for Tuple<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Tuple<'de>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple value")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Tuple::new(vec![]))
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = Value::deserialize(deserializer)?;
                Ok(Tuple::new(vec![value]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut elements = match seq.size_hint() {
                    Some(size) => Vec::with_capacity(size),
                    None => Vec::new(),
                };
                while let Some(element) = seq.next_element()? {
                    elements.push(element);
                }
                Ok(Tuple::new(elements))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut elements = match map.size_hint() {
                    Some(size) => Vec::with_capacity(size),
                    None => Vec::new(),
                };
                while let Some((key, value)) = map.next_entry()? {
                    let element = Value::Tuple(Tuple::new(vec![key, value]));
                    elements.push(element);
                }
                Ok(Tuple::new(elements))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl<'de> Deserialize<'de> for AnnotatedValue<'de> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}
