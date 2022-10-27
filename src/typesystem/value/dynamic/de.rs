pub use super::ser::Error;
use super::{tuple, Dynamic};
use serde::{
    de::{value::MapDeserializer, IntoDeserializer},
    forward_to_deserialize_any,
};

pub fn from_dynamic<T>(d: Dynamic) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(d)
}

impl<'de> serde::Deserializer<'de> for Dynamic {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Dynamic::Void => visitor.visit_unit(),
            Dynamic::Bool(b) => visitor.visit_bool(b),
            Dynamic::Int8(i) => visitor.visit_i8(i),
            Dynamic::UInt8(i) => visitor.visit_u8(i),
            Dynamic::Int16(i) => visitor.visit_i16(i),
            Dynamic::UInt16(i) => visitor.visit_u16(i),
            Dynamic::Int32(i) => visitor.visit_i32(i),
            Dynamic::UInt32(i) => visitor.visit_u32(i),
            Dynamic::Int64(i) => visitor.visit_i64(i),
            Dynamic::UInt64(i) => visitor.visit_u64(i),
            Dynamic::Float(f) => visitor.visit_f32(f),
            Dynamic::Double(d) => visitor.visit_f64(d),
            Dynamic::String(s) => visitor.visit_string(s),
            Dynamic::Raw(buf) => visitor.visit_byte_buf(buf),
            Dynamic::Optional(o) => match o {
                Some(v) => visitor.visit_some(*v),
                None => visitor.visit_none(),
            },
            Dynamic::List(l) => visitor.visit_seq(l.into_deserializer()),
            Dynamic::Map(m) => visitor.visit_map(MapDeserializer::new(m.into_iter())),
            Dynamic::Tuple(_) => todo!(),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_tuple(self, Some(name), visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_tuple(self, Some(name), visitor)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_tuple(self, None, visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_tuple(self, Some(name), visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_tuple(self, Some(name), visitor)
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
        Err(Error::UnionAreNotSupported)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char str string bytes byte_buf option unit
        seq map identifier ignored_any
    }
}

impl<'de> serde::Deserialize<'de> for Dynamic {
    fn deserialize<D>(deserializer: D) -> Result<Dynamic, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Dynamic;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Dynamic, E> {
                Ok(Dynamic::Bool(value))
            }

            fn visit_i8<E>(self, value: i8) -> Result<Dynamic, E> {
                Ok(Dynamic::Int8(value))
            }

            fn visit_u8<E>(self, value: u8) -> Result<Dynamic, E> {
                Ok(Dynamic::UInt8(value))
            }

            fn visit_i16<E>(self, value: i16) -> Result<Dynamic, E> {
                Ok(Dynamic::Int16(value))
            }

            fn visit_u16<E>(self, value: u16) -> Result<Dynamic, E> {
                Ok(Dynamic::UInt16(value))
            }

            fn visit_i32<E>(self, value: i32) -> Result<Dynamic, E> {
                Ok(Dynamic::Int32(value))
            }

            fn visit_u32<E>(self, value: u32) -> Result<Dynamic, E> {
                Ok(Dynamic::UInt32(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Dynamic, E> {
                Ok(Dynamic::Int64(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Dynamic, E> {
                Ok(Dynamic::UInt64(value))
            }

            fn visit_f32<E>(self, value: f32) -> Result<Dynamic, E> {
                Ok(Dynamic::Float(value))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Dynamic, E> {
                Ok(Dynamic::Double(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Dynamic, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(value.to_string())
            }

            fn visit_string<E>(self, value: String) -> Result<Dynamic, E> {
                Ok(Dynamic::String(value))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_byte_buf(v.to_vec())
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E> {
                Ok(Dynamic::Raw(value))
            }

            fn visit_none<E>(self) -> Result<Dynamic, E> {
                Ok(Dynamic::Optional(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Dynamic, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let v = serde::Deserialize::deserialize(deserializer)?;
                Ok(Dynamic::Optional(Some(Box::new(v))))
            }

            fn visit_unit<E>(self) -> Result<Dynamic, E> {
                Ok(Dynamic::Void)
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Ok(Dynamic::Tuple(serde::Deserialize::deserialize(
                    deserializer,
                )?))
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Dynamic, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(Dynamic::List(vec))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Dynamic, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    vec.push(pair);
                }
                Ok(Dynamic::Map(vec))
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::EnumAccess<'de>,
            {
                // TODO ?
                let _ = data;
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Enum,
                    &self,
                ))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for Dynamic {
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

fn deserialize_tuple<'de, V>(
    dynamic: Dynamic,
    name: Option<&str>,
    visitor: V,
) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    use serde::de::Deserializer;
    match dynamic {
        Dynamic::Tuple(tuple) if tuple.name.as_deref() == name => match tuple.elements {
            tuple::Elements::Raw(fields) => visitor.visit_seq(fields.into_deserializer()),
            tuple::Elements::Fields(fields) => visitor.visit_map(MapDeserializer::new(
                fields.into_iter().map(|nf| (nf.name, nf.element)),
            )),
        },
        _ => dynamic.deserialize_any(visitor),
    }
}
