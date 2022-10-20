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
            _ => Err(Error::UnknownValueType),
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

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }

            fn visit_i8<E>(self, value: i8) -> Result<Value, E> {
                Ok(Value::Int8(value))
            }

            fn visit_u8<E>(self, value: u8) -> Result<Value, E> {
                Ok(Value::UInt8(value))
            }

            fn visit_i16<E>(self, value: i16) -> Result<Value, E> {
                Ok(Value::Int16(value))
            }

            fn visit_u16<E>(self, value: u16) -> Result<Value, E> {
                Ok(Value::UInt16(value))
            }

            fn visit_i32<E>(self, value: i32) -> Result<Value, E> {
                Ok(Value::Int32(value))
            }

            fn visit_u32<E>(self, value: u32) -> Result<Value, E> {
                Ok(Value::UInt32(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Int64(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::UInt64(value))
            }

            fn visit_f32<E>(self, value: f32) -> Result<Value, E> {
                Ok(Value::Float(value))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::Double(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(value.to_string())
            }

            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_byte_buf(v.to_vec())
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E> {
                Ok(Value::Raw(value))
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Optional(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let v = serde::Deserialize::deserialize(deserializer)?;
                Ok(Value::Optional(Some(Box::new(v))))
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Void)
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Ok(Value::Tuple(serde::Deserialize::deserialize(deserializer)?))
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Value, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(Value::List(vec))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    vec.push(pair);
                }
                Ok(Value::Map(vec))
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
        deserializer.deserialize_any(ValueVisitor)
    }
}
