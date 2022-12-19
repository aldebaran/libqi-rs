use super::*;
use serde::{
    de::{value::MapDeserializer, value::SeqDeserializer},
    forward_to_deserialize_any,
};

pub fn from_value<'v, T>(d: Value<'v>) -> Result<T, serde::de::value::Error>
where
    T: serde::de::Deserialize<'v>,
{
    T::deserialize(d)
}

pub fn from_value_ref<'v, T>(d: &'v Value<'v>) -> Result<T, serde::de::value::Error>
where
    T: serde::Deserialize<'v>,
{
    T::deserialize(d)
}

impl<'de> serde::Deserializer<'de> for Value<'de> {
    type Error = serde::de::value::Error;

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
        use serde::de::IntoDeserializer;
        match self {
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Number(n) => n.deserialize_any(visitor),
            Value::String(s) => s.deserialize_any(visitor),
            Value::Raw(buf) => match buf.into() {
                Cow::Borrowed(bytes) => visitor.visit_borrowed_bytes(bytes),
                Cow::Owned(buf) => visitor.visit_byte_buf(buf),
            },
            Value::Option(option) => match *option {
                Some(v) => visitor.visit_some(v.into_deserializer()),
                None => visitor.visit_none(),
            },
            Value::List(elements) => {
                SeqDeserializer::new(elements.into_iter()).deserialize_any(visitor)
            }
            Value::Tuple(tuple) => match tuple.len() {
                0 => visitor.visit_unit(),
                1 => visitor.visit_newtype_struct(tuple.into_iter().next().unwrap()),
                _ => visitor.visit_seq(SeqDeserializer::new(tuple.into_iter())),
            },
            Value::Map(map) => MapDeserializer::new(map.into_iter()).deserialize_any(visitor),
        }
    }
}

impl<'de> serde::Deserializer<'de> for &'de Value<'de> {
    type Error = serde::de::value::Error;

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
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Number(n) => n.deserialize_any(visitor),
            Value::String(s) => s.deserialize_any(visitor),
            Value::Raw(buf) => visitor.visit_borrowed_bytes(buf.as_ref()),
            Value::Option(option) => match option.as_ref() {
                Some(v) => visitor.visit_some(v),
                None => visitor.visit_none(),
            },
            Value::List(elements) => visitor.visit_seq(SeqDeserializer::new(elements.iter())),
            Value::Tuple(tuple) => match tuple.len() {
                0 => visitor.visit_unit(),
                1 => visitor.visit_newtype_struct(&tuple[0]),
                _ => visitor.visit_seq(SeqDeserializer::new(tuple.iter())),
            },
            Value::Map(Map(map)) => {
                visitor.visit_map(MapDeserializer::new(map.iter().map(|(k, v)| (k, v))))
            }
        }
    }
}

impl<'de> serde::de::IntoDeserializer<'de, serde::de::value::Error> for Value<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> serde::de::IntoDeserializer<'de, serde::de::value::Error> for &'de Value<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
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
                Ok(Value::from(v))
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Number::from(v)))
            }

            fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let str = v.encode_utf8(&mut [0; 4]).to_owned();
                Ok(Value::from(String::from(str)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(String::from(v.to_owned())))
            }

            fn visit_string<E>(self, v: std::string::String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(String::from(v)))
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
                Ok(Value::from(Raw::from(v.to_owned())))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Raw::from(v)))
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(Raw::from(v)))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::from(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = Value::deserialize(deserializer)?;
                Ok(Value::from(Some(value)))
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
                Ok(Value::from(list))
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
                Ok(Value::from(Map::new(map_vec)))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::unit())
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
                Ok(Value::from(Tuple::new(vec![value])))
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

impl<'de> Deserialize<'de> for AnnotatedValue<'de> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
        //struct Visitor;
        //#[derive(serde::Deserialize)]
        //#[serde(rename_all = "snake_case")]
        //enum Fields {
        //    Signature,
        //    Value,
        //}
        //const FIELDS: [&str; 2] = ["signature", "value"];
        //impl<'de> serde::de::Visitor<'de> for Visitor {
        //    type Value = AnnotatedValue<'de>;

        //    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        //        formatter.write_str("an annotated value")
        //    }

        //    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        //    where
        //        A: serde::de::MapAccess<'de>,
        //    {
        //        let signature: Option<Signature> = None;
        //        let value;
        //        while let Some((key, value)) = map.next_entry()? {
        //            match key {
        //                Fields::Signature => signature = value,
        //                Fields::Value => ,
        //            }
        //        }
        //        debug_assert!(signature.type(), value.get_type());
        //        Ok(AnnotatedValue { signature.into_type(), value })
        //    }
        //}
        //deserializer.deserialize_struct("AnnotatedValue", &FIELDS, Visitor)
    }
}
