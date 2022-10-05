use super::{tuple, Error, Value};
use serde::{
    de::{value::MapDeserializer, IntoDeserializer},
    forward_to_deserialize_any,
};

fn deserialize_tuple<'de, V>(
    value: Value,
    name: Option<&str>,
    visitor: V,
) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    use serde::de::Deserializer;
    match value {
        Value::Tuple(tuple) if tuple.name.as_deref() == name => match tuple.fields {
            tuple::Fields::Unnamed(fields) => visitor.visit_seq(fields.into_deserializer()),
            tuple::Fields::Named(fields) => visitor.visit_map(MapDeserializer::new(
                fields.into_iter().map(|nf| (nf.name, nf.value)),
            )),
        },
        _ => value.deserialize_any(visitor),
    }
}

impl<'de> serde::Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Void => visitor.visit_unit(),
            Value::Int8(i) => visitor.visit_i8(i),
            Value::UInt8(i) => visitor.visit_u8(i),
            Value::Int16(i) => visitor.visit_i16(i),
            Value::UInt16(i) => visitor.visit_u16(i),
            Value::Int32(i) => visitor.visit_i32(i),
            Value::UInt32(i) => visitor.visit_u32(i),
            Value::Int64(i) => visitor.visit_i64(i),
            Value::UInt64(i) => visitor.visit_u64(i),
            Value::Float(f) => visitor.visit_f32(f),
            Value::Double(d) => visitor.visit_f64(d),
            Value::String(s) => visitor.visit_string(s),
            Value::List(l) => visitor.visit_seq(l.into_deserializer()),
            Value::Map(m) => visitor.visit_map(MapDeserializer::new(m.into_iter())),
            Value::Raw(buf) => visitor.visit_byte_buf(buf),
            Value::Optional(o) => match o {
                Some(v) => visitor.visit_some(*v),
                None => visitor.visit_none(),
            },
            _ => Err(Error::ValueCannotBeDeserialized),
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
