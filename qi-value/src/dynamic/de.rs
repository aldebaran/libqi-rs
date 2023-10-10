use crate::object::deserialize_object;
use qi_type::{Signature, Tuple, Type};
use serde::forward_to_deserialize_any;
use std::marker::PhantomData;

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    deserialize_with_visitor(deserializer, Visitor::new())
}

fn deserialize_with_visitor<'de, D, V>(deserializer: D, visitor: V) -> Result<V::Value, D::Error>
where
    D: serde::Deserializer<'de>,
    V: serde::de::Visitor<'de>,
{
    deserializer.deserialize_struct("Dynamic", &["signature", "value"], visitor)
}

#[derive(Debug)]
struct Visitor<T>(PhantomData<T>);

impl<T> Visitor<T> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de, T> serde::de::Visitor<'de> for Visitor<T>
where
    T: serde::Deserialize<'de>,
{
    type Value = T;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a Dynamic value")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        // Signature
        let signature: Signature = seq
            .next_element()?
            .ok_or_else(|| Error::invalid_length(0, &self))?;
        let value_type = signature.into_type();

        // Value
        let value = seq
            .next_element_seed(ValueSeed::new(value_type))?
            .ok_or_else(|| Error::invalid_length(1, &self))?;

        Ok(value)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Signature,
            Value,
        }
        use serde::de::Error;

        let signature: Signature = match map.next_key()? {
            Some(Field::Signature) => map.next_value(),
            _ => Err(Error::missing_field("signature")),
        }?;
        let value_type = signature.into_type();
        let value = match map.next_key()? {
            Some(Field::Value) => map.next_value_seed(ValueSeed::new(value_type)),
            _ => Err(Error::missing_field("value")),
        }?;
        Ok(value)
    }
}

#[derive(Debug)]
struct ValueSeed<T> {
    ty: Option<Type>,
    phantom: PhantomData<T>,
}

impl<T> ValueSeed<T> {
    fn new(ty: Option<Type>) -> Self {
        Self {
            ty,
            phantom: PhantomData,
        }
    }
}

impl<'de, T> serde::de::DeserializeSeed<'de> for ValueSeed<T>
where
    T: serde::Deserialize<'de>,
{
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(TypeDeserializer {
            ty: self.ty,
            deserializer,
        })
    }
}

#[derive(Debug)]
struct TypeDeserializer<D> {
    ty: Option<Type>,
    deserializer: D,
}

impl<'de, D> serde::Deserializer<'de> for TypeDeserializer<D>
where
    D: serde::Deserializer<'de>,
{
    type Error = D::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let TypeDeserializer { ty, deserializer } = self;
        match ty {
            Some(Type::Unit) => deserializer.deserialize_unit(visitor),
            Some(Type::Bool) => deserializer.deserialize_bool(visitor),
            Some(Type::Int8) => deserializer.deserialize_i8(visitor),
            Some(Type::UInt8) => deserializer.deserialize_u8(visitor),
            Some(Type::Int16) => deserializer.deserialize_i16(visitor),
            Some(Type::UInt16) => deserializer.deserialize_u16(visitor),
            Some(Type::Int32) => deserializer.deserialize_i32(visitor),
            Some(Type::UInt32) => deserializer.deserialize_u32(visitor),
            Some(Type::Int64) => deserializer.deserialize_i64(visitor),
            Some(Type::UInt64) => deserializer.deserialize_u64(visitor),
            Some(Type::Float32) => deserializer.deserialize_f32(visitor),
            Some(Type::Float64) => deserializer.deserialize_f64(visitor),
            Some(Type::String) => deserializer.deserialize_str(visitor),
            Some(Type::Raw) => deserializer.deserialize_bytes(visitor),
            Some(Type::Object) => deserialize_object(deserializer, visitor),
            Some(Type::Option(_)) => deserializer.deserialize_option(visitor),
            Some(Type::List(_) | Type::VarArgs(_)) => deserializer.deserialize_seq(visitor),
            Some(Type::Map { .. }) => deserializer.deserialize_map(visitor),
            Some(Type::Tuple(Tuple::Tuple(elems))) => {
                deserializer.deserialize_tuple(elems.len(), visitor)
            }
            Some(Type::Tuple(Tuple::TupleStruct { elements, .. })) => {
                deserializer.deserialize_tuple(elements.len(), visitor)
            }
            Some(Type::Tuple(Tuple::Struct { fields, .. })) => {
                deserializer.deserialize_tuple(fields.len(), visitor)
            }
            None => deserialize_with_visitor(deserializer, visitor),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
