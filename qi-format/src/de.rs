/// Deserialization of `serde` values from the `qi` format.
///
/// The following `serde` types are not handled:
///
/// - `i128`
/// - `u128`
/// - `any`
/// - `ignored any`
///
/// Identifiers are deserialized as unit values.
use crate::{read, Error, Result};
use bytes::Buf;
use qi_value as value;
use sealed::sealed;
use serde::de::IntoDeserializer;

pub fn from_buf<'de, B, T>(mut buf: B) -> Result<T>
where
    T: serde::de::Deserialize<'de>,
    B: Buf,
{
    T::deserialize(Deserializer::from_buf(&mut buf))
}

#[sealed]
pub trait BufExt: Buf {
    fn deserialize_value_of_type(
        &mut self,
        value_type: Option<&value::Type>,
    ) -> Result<value::Value<'static>>;

    fn deserialize_value<T>(&mut self) -> Result<T>
    where
        T: value::Reflect + value::FromValue<'static>;
}

#[sealed]
impl<B> BufExt for B
where
    B: Buf,
{
    fn deserialize_value_of_type(
        &mut self,
        value_type: Option<&value::Type>,
    ) -> Result<value::Value<'static>> {
        value::deserialize_value_of_type(Deserializer::from_buf(self), value_type)
    }

    fn deserialize_value<T>(&mut self) -> Result<T>
    where
        T: value::Reflect + value::FromValue<'static>,
    {
        let value = self.deserialize_value_of_type(T::ty().as_ref())?;
        Ok(value.cast()?)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deserializer<'b, B> {
    buf: &'b mut B,
}

impl<'b, B> Deserializer<'b, B> {
    pub fn from_buf(buf: &'b mut B) -> Self {
        Self { buf }
    }
}

impl<'b, 'de, B> serde::Deserializer<'de> for Deserializer<'b, B>
where
    B: Buf,
{
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(Error::CannotDeserializeAny)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bool(read::read_bool(self.buf)?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i8(read::read_i8(self.buf)?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i16(read::read_i16(self.buf)?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i32(read::read_i32(self.buf)?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i64(read::read_i64(self.buf)?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u8(read::read_u8(self.buf)?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u16(read::read_u16(self.buf)?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u32(read::read_u32(self.buf)?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u64(read::read_u64(self.buf)?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_f32(read::read_f32(self.buf)?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_f64(read::read_f64(self.buf)?)
    }

    // equivalence char -> str
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_string(read::read_string(self.buf)?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bytes(&read::read_raw(self.buf)?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_byte_buf(read::read_raw_buf(self.buf)?)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match read::read_bool(self.buf)? {
            true => visitor.visit_some(self),
            false => visitor.visit_none(),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        // nothing
        visitor.visit_unit()
    }

    // equivalence: unit_struct -> unit
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // equivalence: newtype_struct(T) = tuple(T) = T
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let access = SequenceAccess::new_list_or_map(self.buf)?;
        visitor.visit_seq(access)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let access = SequenceAccess::new_sequence(len, self.buf);
        visitor.visit_seq(access)
    }

    // equivalence: tuple_struct(T...) -> tuple(T...)
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let access = SequenceAccess::new_list_or_map(self.buf)?;
        visitor.visit_map(access)
    }

    // equivalence: struct(T...) -> tuple(T...)
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_tuple(fields.len(), visitor)
    }

    // equivalence: enum(idx,T) -> tuple(idx,T)
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_enum(self)
    }

    // equivalence: identifier -> unit
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(Error::CannotDeserializeAny)
    }
}

impl<'b, 'de, B> serde::de::EnumAccess<'de> for Deserializer<'b, B>
where
    B: Buf,
{
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant_index = read::read_u32(self.buf)?;
        let variant_index_deserializer = variant_index.into_deserializer();
        let value: Result<_> = seed.deserialize(variant_index_deserializer);
        Ok((value?, self))
    }
}

impl<'b, 'de, B> serde::de::VariantAccess<'de> for Deserializer<'b, B>
where
    B: Buf,
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        use serde::Deserializer;
        self.deserialize_tuple(len, visitor)
    }

    // equivalence: struct(T...) -> tuple(T...)
    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        use serde::Deserializer;
        self.deserialize_tuple(fields.len(), visitor)
    }
}

struct SequenceAccess<'b, B> {
    iter: std::ops::Range<usize>,
    buf: &'b mut B,
}

impl<'b, 'de, B> SequenceAccess<'b, B>
where
    B: Buf,
{
    fn new_list_or_map(buf: &'b mut B) -> Result<Self> {
        let size = read::read_size(buf)?;
        Ok(Self::new_sequence(size, buf))
    }

    fn new_sequence(size: usize, buf: &'b mut B) -> Self {
        Self { iter: 0..size, buf }
    }

    fn next_item<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        let item = match self.iter.next() {
            Some(_idx) => {
                let item = seed.deserialize(Deserializer::from_buf(self.buf))?;
                Some(item)
            }
            None => None,
        };
        Ok(item)
    }
}

impl<'b, 'de, B> serde::de::SeqAccess<'de> for SequenceAccess<'b, B>
where
    B: Buf,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.next_item(seed)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

impl<'b, 'de, B> serde::de::MapAccess<'de> for SequenceAccess<'b, B>
where
    B: Buf,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        self.next_item(seed)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(Deserializer::from_buf(self.buf))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

impl serde::de::Error for super::Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Deserializer as D;
    use super::*;
    use assert_matches::assert_matches;
    use serde::de::{Deserialize, Deserializer};
    use serde_value::{Value, ValueVisitor};

    // --------------------------------------------------------------
    // Bijection types
    // --------------------------------------------------------------
    // To simplify some tests, we deserialize concrete types directly.
    // We assume that:
    // - `std::option::Option` uses `deserialize_option`.
    // - `std::vec::Vec` uses `deserialize_seq`.
    // - tuples and arrays use `deserialize_tuple`.
    // - `std::collections::HashMap` uses `deserialize_map`.
    // - a struct/newtype struct/unit struct/tuple struct with derived Deserialize implementation
    // uses `deserialize_struct/newtype_struct/unit_struct/tuple_struct`.
    // - enums use `deserializer_enum`.

    #[test]
    fn test_deserializer_deserialize_bool() {
        let mut buf: &[u8] = &[0, 1, 2];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_bool(ValueVisitor),
            Ok(Value::Bool(false))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_bool(ValueVisitor),
            Ok(Value::Bool(true))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_bool(ValueVisitor),
            Err(Error::NotABoolValue(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_bool(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_i8() {
        let mut buf: &[u8] = &[1, 2];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i8(ValueVisitor),
            Ok(Value::I8(1))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i8(ValueVisitor),
            Ok(Value::I8(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i8(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_u8() {
        let mut buf: &[u8] = &[1, 2];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u8(ValueVisitor),
            Ok(Value::U8(1))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u8(ValueVisitor),
            Ok(Value::U8(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u8(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_i16() {
        let mut buf: &[u8] = &[1, 0, 2, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i16(ValueVisitor),
            Ok(Value::I16(1))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i16(ValueVisitor),
            Ok(Value::I16(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i16(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_u16() {
        let mut buf: &[u8] = &[1, 0, 2, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u16(ValueVisitor),
            Ok(Value::U16(1))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u16(ValueVisitor),
            Ok(Value::U16(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u16(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_i32() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 2, 0, 0, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i32(ValueVisitor),
            Ok(Value::I32(1))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i32(ValueVisitor),
            Ok(Value::I32(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i32(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_u32() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 2, 0, 0, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u32(ValueVisitor),
            Ok(Value::U32(1))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u32(ValueVisitor),
            Ok(Value::U32(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u32(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_i64() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i64(ValueVisitor),
            Ok(Value::I64(1))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i64(ValueVisitor),
            Ok(Value::I64(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i64(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_u64() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u64(ValueVisitor),
            Ok(Value::U64(1))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u64(ValueVisitor),
            Ok(Value::U64(2))
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u64(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_deserializer_deserialize_f32() {
        let mut buf: &[u8] = &[0x14, 0xae, 0x29, 0x42, 0xff, 0xff, 0xff, 0x7f];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_f32(ValueVisitor),
            Ok(Value::F32(f)) => assert_eq!(f, 42.42)
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_f32(ValueVisitor),
            Ok(Value::F32(f)) => assert!(f.is_nan())
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_f32(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_f64() {
        let mut buf: &[u8] = &[
            0xf6, 0x28, 0x5c, 0x8f, 0xc2, 0x35, 0x45, 0x40, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0x7f,
        ];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_f64(ValueVisitor),
            Ok(Value::F64(f)) => assert_eq!(f, 42.42)
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_f64(ValueVisitor),
            Ok(Value::F64(f)) => assert!(f.is_nan())
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_f64(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_bytes() {
        let mut buf: &[u8] = &[4, 0, 0, 0, 1, 2, 3, 4, 0, 0, 0, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_bytes(ValueVisitor),
            Ok(Value::Bytes(v)) => assert_eq!(v, [1, 2, 3, 4])
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_bytes(ValueVisitor),
            Ok(Value::Bytes(v)) => assert!(v.is_empty())
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_bytes(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_bytes_missing_elements() {
        let mut buf: &[u8] = &[4, 0, 0, 0, 1, 2, 3];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_bytes(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_option() {
        let mut buf: &[u8] = &[1, 42, 0, 0];
        assert_matches!(
            std::option::Option::<i16>::deserialize(D::from_buf(&mut buf)),
            Ok(Some(42i16))
        );
        assert_matches!(
            std::option::Option::<i16>::deserialize(D::from_buf(&mut buf)),
            Ok(None)
        );
        assert_matches!(
            std::option::Option::<i16>::deserialize(D::from_buf(&mut buf)),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_unit() {
        let mut buf: &[u8] = &[];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_unit(ValueVisitor),
            Ok(Value::Unit)
        );
    }

    #[test]
    fn test_deserializer_deserialize_sequence() {
        let mut buf: &[u8] = &[3, 0, 0, 0, 1, 0, 2, 0, 3, 0, 0, 0, 0, 0];
        assert_matches!(
            std::vec::Vec::<i16>::deserialize(D::from_buf(&mut buf)),
            Ok(v) => assert_eq!(v, [1, 2, 3])
        );
        assert_matches!(
            std::vec::Vec::<i16>::deserialize(D::from_buf(&mut buf)),
            Ok(v) => assert!(v.is_empty())
        );
        assert_matches!(
            std::vec::Vec::<i16>::deserialize(D::from_buf(&mut buf)),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_sequence_missing_elements() {
        let mut buf: &[u8] = &[2, 0, 0, 0, 1, 2];
        assert_matches!(
            std::vec::Vec::<i16>::deserialize(D::from_buf(&mut buf)),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_tuple() {
        let mut buf: &[u8] = &[2, 0, 0, 0, 1, 2];
        assert_matches!(
            <(u32, Option<i8>)>::deserialize(D::from_buf(&mut buf)),
            Ok((2, Some(2)))
        );
        assert_matches!(
            <(u32, Option<i8>)>::deserialize(D::from_buf(&mut buf)),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_map() {
        let mut buf: &[u8] = &[2, 0, 0, 0, 1, 2, 2, 4];
        use std::collections::HashMap;
        assert_matches!(
            HashMap::<i8, u8>::deserialize(D::from_buf(&mut buf)),
            Ok(m) => assert_eq!(m, HashMap::from([(1, 2), (2, 4)]))
        );
        assert_matches!(
            HashMap::<i8, u8>::deserialize(D::from_buf(&mut buf)),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_map_missing_elements() {
        let mut buf: &[u8] = &[2, 0, 0, 0, 1, 2];
        use std::collections::HashMap;
        assert_matches!(
            HashMap::<i8, u8>::deserialize(D::from_buf(&mut buf)),
            Err(Error::ShortRead)
        );
    }

    // --------------------------------------------------------------
    // Equivalence types
    // --------------------------------------------------------------
    #[test]
    // char -> str
    fn test_deserializer_deserialize_char() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99];
        // `deserialize_char` yields strings, the visitor decides if it handles them or not.
        assert_matches!(
            D::from_buf(&mut buf).deserialize_char(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "a")
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_char(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "bc")
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_char(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    // str -> raw
    fn test_deserializer_deserialize_str() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 3, 0, 0, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_str(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "a")
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_str(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "bc")
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_str(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    // string -> raw
    fn test_deserializer_deserialize_string() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 0, 0, 0, 0, 3, 0, 0, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_string(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "a")
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_string(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "bc")
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_string(ValueVisitor),
            Ok(Value::String(s)) => assert!(s.is_empty())
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_string(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_deserializer_deserialize_byte_buf() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 0, 0, 0, 0, 3, 0, 0, 0];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert_eq!(b, [97])
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert_eq!(b, [98, 99])
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert!(b.is_empty())
        );
        assert_matches!(
            D::from_buf(&mut buf).deserialize_byte_buf(ValueVisitor),
            Err(Error::ShortRead)
        );
    }

    #[test]
    // struct(T...) -> tuple(T...)
    fn test_deserializer_deserialize_struct() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 0, 0, 0, 0, 3, 0, 0, 0];
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S {
            c: char,
            s: std::string::String,
            t: (u8, i8, i16),
            i: i32,
        }
        assert_matches!(
            S::deserialize(D::from_buf(&mut buf)),
            Ok(S {
                c: 'a',
                s,
                t: (0, 0, 0),
                i: 3
            }) => assert_eq!(s, "bc")
        );
        assert_matches!(S::deserialize(D::from_buf(&mut buf)), Err(Error::ShortRead));
    }

    #[test]
    // newtype_struct(T) -> tuple(T) = T
    fn test_deserializer_deserialize_newtype_struct() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 97, 1, 0, 0, 0, 98];
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S(char);
        assert_matches!(S::deserialize(D::from_buf(&mut buf)), Ok(S('a')));
        assert_matches!(S::deserialize(D::from_buf(&mut buf)), Ok(S('b')));
        assert_matches!(S::deserialize(D::from_buf(&mut buf)), Err(Error::ShortRead));
    }

    #[test]
    // unit_struct -> unit
    fn test_deserializer_deserialize_unit_struct() {
        let mut buf: &[u8] = &[];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_unit_struct("MyStruct", ValueVisitor),
            Ok(Value::Unit)
        );
    }

    #[test]
    // tuple_struct(T...) -> tuple(T...)
    fn test_deserializer_deserialize_tuple_struct() {
        let mut buf: &[u8] = &[1, 0, 0, 0, 97, 3, 0, 0, 0, 4, 0, 5, 0, 6, 0];
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S(String, Vec<i16>);
        assert_matches!(S::deserialize(D::from_buf(&mut buf)), Ok(S(str, v)) => {
            assert_eq!(str, "a");
            assert_eq!(v, [4, 5, 6])
        });
    }

    #[test]
    // enum(idx,T) -> tuple(idx,T)
    fn test_deserializer_deserialize_enum() {
        let mut buf: &[u8] = &[
            0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 97, 3, 0, 0, 0, 4, 0, 5, 0, 6, 0,
        ];
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        enum E {
            A,
            B(char),
            C,
            D(i16, i16, i16),
        }
        assert_matches!(E::deserialize(D::from_buf(&mut buf)), Ok(E::A));
        assert_matches!(E::deserialize(D::from_buf(&mut buf)), Ok(E::B('a')));
        assert_matches!(E::deserialize(D::from_buf(&mut buf)), Ok(E::D(4, 5, 6)));
        assert_matches!(E::deserialize(D::from_buf(&mut buf)), Err(Error::ShortRead));
    }

    #[test]
    // identifier => unit
    fn test_deserializer_deserialize_identifier() {
        let mut buf: &[u8] = &[];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_identifier(ValueVisitor),
            Ok(Value::Unit)
        );
    }

    // --------------------------------------------------------------
    // Unhandled types
    // --------------------------------------------------------------
    #[test]
    fn test_deserializer_deserialize_i128() {
        let mut buf: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_i128(ValueVisitor),
            Err(Error::Custom(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_u128() {
        let mut buf: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_u128(ValueVisitor),
            Err(Error::Custom(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_any() {
        let mut buf: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_any(ValueVisitor),
            Err(Error::CannotDeserializeAny)
        );
    }

    #[test]
    fn test_deserializer_deserialize_ignored_any() {
        let mut buf: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        assert_matches!(
            D::from_buf(&mut buf).deserialize_any(ValueVisitor),
            Err(Error::CannotDeserializeAny)
        );
    }
}
