use crate::{read, Error, Result, Value};
use qi_value::Raw;
use serde::de::IntoDeserializer;

pub fn from_value<'v, T>(value: &'v Value) -> Result<T>
where
    T: serde::de::Deserialize<'v>,
{
    let mut de = Deserializer::from_slice(value.as_bytes());
    T::deserialize(&mut de)
}

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Deserializer<R> {
    reader: R,
}

impl<R> Deserializer<R>
where
    R: read::Read,
{
    fn from_reader(reader: R) -> Self {
        Self { reader }
    }

    fn as_ref(&mut self) -> &mut Self {
        self
    }
}

impl<R> Deserializer<read::IoRead<R>>
where
    R: std::io::Read,
{
    pub fn from_io_reader(reader: R) -> Self {
        Self::from_reader(read::IoRead::new(reader))
    }
}

impl<'b> Deserializer<read::SliceRead<'b>> {
    pub fn from_slice(data: &'b [u8]) -> Self {
        Self::from_reader(read::SliceRead::new(data))
    }
}

trait StrDeserializer<'de> {
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>;

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>;
}

impl<'de> StrDeserializer<'de> for &'de str {
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_string(self.to_owned())
    }
}

impl<'de> StrDeserializer<'de> for String {
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_str(&self)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_string(self)
    }
}

trait BytesDeserializer<'de> {
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>;

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>;
}

impl<'de> BytesDeserializer<'de> for &'de [u8] {
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_byte_buf(self.to_vec())
    }
}

impl<'de> BytesDeserializer<'de> for Raw {
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bytes(&self)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_byte_buf(self.to_vec())
    }
}

impl<'de, R> serde::Deserializer<'de> for &mut Deserializer<R>
where
    R: read::Read,
    R::Raw: BytesDeserializer<'de>,
    R::Str: StrDeserializer<'de>,
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
        visitor.visit_bool(self.reader.read_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i8(self.reader.read_i8()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i16(self.reader.read_i16()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i32(self.reader.read_i32()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i64(self.reader.read_i64()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u8(self.reader.read_u8()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u16(self.reader.read_u16()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u32(self.reader.read_u32()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u64(self.reader.read_u64()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_f32(self.reader.read_f32()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_f64(self.reader.read_f64()?)
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
        let str = self.reader.read_str()?;
        str.deserialize_str(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let str = self.reader.read_str()?;
        str.deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let raw = self.reader.read_raw()?;
        raw.deserialize_bytes(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let raw = self.reader.read_raw()?;
        raw.deserialize_byte_buf(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.reader.read_bool()? {
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
        let access = SequenceAccess::new_list_or_map(self)?;
        visitor.visit_seq(access)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let access = SequenceAccess::new_sequence(len, self);
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
        let access = SequenceAccess::new_list_or_map(self)?;
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

impl<'de, R> serde::de::EnumAccess<'de> for &mut Deserializer<R>
where
    R: read::Read,
    Self: serde::Deserializer<'de, Error = Error>,
{
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant_index = self.reader.read_u32()?;
        let variant_index_deserializer = variant_index.into_deserializer();
        let value: Result<_> = seed.deserialize(variant_index_deserializer);
        Ok((value?, self))
    }
}

impl<'de, R> serde::de::VariantAccess<'de> for &mut Deserializer<R>
where
    Self: serde::Deserializer<'de, Error = Error>,
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

struct SequenceAccess<'a, R> {
    iter: std::ops::Range<usize>,
    deserializer: &'a mut Deserializer<R>,
}

impl<'a, 'de, R> SequenceAccess<'a, R>
where
    R: read::Read,
    for<'d> &'d mut Deserializer<R>: serde::Deserializer<'de, Error = Error>,
{
    fn new_list_or_map(deserializer: &'a mut Deserializer<R>) -> Result<Self> {
        let size = deserializer.reader.read_size()?;
        Ok(Self::new_sequence(size, deserializer))
    }

    fn new_sequence(size: usize, deserializer: &'a mut Deserializer<R>) -> Self {
        Self {
            iter: 0..size,
            deserializer,
        }
    }

    fn next_item<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        let item = match self.iter.next() {
            Some(_idx) => {
                let item = seed.deserialize(self.deserializer.as_ref())?;
                Some(item)
            }
            None => None,
        };
        Ok(item)
    }
}

impl<'a, 'de, R> serde::de::SeqAccess<'de> for SequenceAccess<'a, R>
where
    R: read::Read,
    for<'d> &'d mut Deserializer<R>: serde::Deserializer<'de, Error = Error>,
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

impl<'a, 'de, R> serde::de::MapAccess<'de> for SequenceAccess<'a, R>
where
    R: read::Read,
    for<'d> &'d mut Deserializer<R>: serde::Deserializer<'de, Error = Error>,
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
        seed.deserialize(self.deserializer.as_ref())
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
        let data = [0, 1, 2];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_bool(ValueVisitor),
            Ok(Value::Bool(false))
        );
        assert_matches!(
            deserializer.deserialize_bool(ValueVisitor),
            Ok(Value::Bool(true))
        );
        assert_matches!(
            deserializer.deserialize_bool(ValueVisitor),
            Err(Error::NotABoolValue(2))
        );
        assert_matches!(
            deserializer.deserialize_bool(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_i8() {
        let data = [1, 2];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(deserializer.deserialize_i8(ValueVisitor), Ok(Value::I8(1)));
        assert_matches!(deserializer.deserialize_i8(ValueVisitor), Ok(Value::I8(2)));
        assert_matches!(deserializer.deserialize_i8(ValueVisitor), Err(Error::Io(_)));
    }

    #[test]
    fn test_deserializer_deserialize_u8() {
        let data = [1, 2];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(deserializer.deserialize_u8(ValueVisitor), Ok(Value::U8(1)));
        assert_matches!(deserializer.deserialize_u8(ValueVisitor), Ok(Value::U8(2)));
        assert_matches!(deserializer.deserialize_u8(ValueVisitor), Err(Error::Io(_)));
    }

    #[test]
    fn test_deserializer_deserialize_i16() {
        let data = [1, 0, 2, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_i16(ValueVisitor),
            Ok(Value::I16(1))
        );
        assert_matches!(
            deserializer.deserialize_i16(ValueVisitor),
            Ok(Value::I16(2))
        );
        assert_matches!(
            deserializer.deserialize_i16(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_u16() {
        let data = [1, 0, 2, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_u16(ValueVisitor),
            Ok(Value::U16(1))
        );
        assert_matches!(
            deserializer.deserialize_u16(ValueVisitor),
            Ok(Value::U16(2))
        );
        assert_matches!(
            deserializer.deserialize_u16(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_i32() {
        let data = [1, 0, 0, 0, 2, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_i32(ValueVisitor),
            Ok(Value::I32(1))
        );
        assert_matches!(
            deserializer.deserialize_i32(ValueVisitor),
            Ok(Value::I32(2))
        );
        assert_matches!(
            deserializer.deserialize_i32(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_u32() {
        let data = [1, 0, 0, 0, 2, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_u32(ValueVisitor),
            Ok(Value::U32(1))
        );
        assert_matches!(
            deserializer.deserialize_u32(ValueVisitor),
            Ok(Value::U32(2))
        );
        assert_matches!(
            deserializer.deserialize_u32(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_i64() {
        let data = [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_i64(ValueVisitor),
            Ok(Value::I64(1))
        );
        assert_matches!(
            deserializer.deserialize_i64(ValueVisitor),
            Ok(Value::I64(2))
        );
        assert_matches!(
            deserializer.deserialize_i64(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_u64() {
        let data = [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_u64(ValueVisitor),
            Ok(Value::U64(1))
        );
        assert_matches!(
            deserializer.deserialize_u64(ValueVisitor),
            Ok(Value::U64(2))
        );
        assert_matches!(
            deserializer.deserialize_u64(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_deserializer_deserialize_f32() {
        let data = [0x14, 0xae, 0x29, 0x42, 0xff, 0xff, 0xff, 0x7f];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_f32(ValueVisitor),
            Ok(Value::F32(f)) => assert_eq!(f, 42.42)
        );
        assert_matches!(
            deserializer.deserialize_f32(ValueVisitor),
            Ok(Value::F32(f)) => assert!(f.is_nan())
        );
        assert_matches!(
            deserializer.deserialize_f32(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_f64() {
        let data = [
            0xf6, 0x28, 0x5c, 0x8f, 0xc2, 0x35, 0x45, 0x40, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0x7f,
        ];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_f64(ValueVisitor),
            Ok(Value::F64(f)) => assert_eq!(f, 42.42)
        );
        assert_matches!(
            deserializer.deserialize_f64(ValueVisitor),
            Ok(Value::F64(f)) => assert!(f.is_nan())
        );
        assert_matches!(
            deserializer.deserialize_f64(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_bytes() {
        let data = [4, 0, 0, 0, 1, 2, 3, 4, 0, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_bytes(ValueVisitor),
            Ok(Value::Bytes(v)) => assert_eq!(v, [1, 2, 3, 4])
        );
        assert_matches!(
            deserializer.deserialize_bytes(ValueVisitor),
            Ok(Value::Bytes(v)) => assert!(v.is_empty())
        );
        assert_matches!(
            deserializer.deserialize_bytes(ValueVisitor),
            Err(Error::Io(_))
        );

        // Bytes may be borrowed from slice
        let data = [4, 0, 0, 0, 1, 2, 3, 4];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(<&[u8]>::deserialize(&mut deserializer), Ok([1, 2, 3, 4]));
    }

    #[test]
    fn test_deserializer_deserialize_bytes_missing_elements() {
        let data = [4, 0, 0, 0, 1, 2, 3];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_bytes(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_option() {
        let data = [1, 42, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            std::option::Option::<i16>::deserialize(&mut deserializer),
            Ok(Some(42i16))
        );
        assert_matches!(
            std::option::Option::<i16>::deserialize(&mut deserializer),
            Ok(None)
        );
        assert_matches!(
            std::option::Option::<i16>::deserialize(&mut deserializer),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_unit() {
        let data = [];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(deserializer.deserialize_unit(ValueVisitor), Ok(Value::Unit));
    }

    #[test]
    fn test_deserializer_deserialize_sequence() {
        let data = [3, 0, 0, 0, 1, 0, 2, 0, 3, 0, 0, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            std::vec::Vec::<i16>::deserialize(&mut deserializer),
            Ok(v) => assert_eq!(v, [1, 2, 3])
        );
        assert_matches!(
            std::vec::Vec::<i16>::deserialize(&mut deserializer),
            Ok(v) => assert!(v.is_empty())
        );
        assert_matches!(
            std::vec::Vec::<i16>::deserialize(&mut deserializer),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_sequence_missing_elements() {
        let data = [2, 0, 0, 0, 1, 2];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            std::vec::Vec::<i16>::deserialize(&mut deserializer),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_tuple() {
        let data = [2, 0, 0, 0, 1, 2];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            <(u32, Option<i8>)>::deserialize(&mut deserializer),
            Ok((2, Some(2)))
        );
        assert_matches!(
            <(u32, Option<i8>)>::deserialize(&mut deserializer),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_map() {
        let data = [2, 0, 0, 0, 1, 2, 2, 4];
        let mut deserializer = super::Deserializer::from_slice(&data);
        use std::collections::HashMap;
        assert_matches!(
            HashMap::<i8, u8>::deserialize(&mut deserializer),
            Ok(m) => assert_eq!(m, HashMap::from([(1, 2), (2, 4)]))
        );
        assert_matches!(
            HashMap::<i8, u8>::deserialize(&mut deserializer),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_map_missing_elements() {
        let data = [2, 0, 0, 0, 1, 2];
        let mut deserializer = super::Deserializer::from_slice(&data);
        use std::collections::HashMap;
        assert_matches!(
            HashMap::<i8, u8>::deserialize(&mut deserializer),
            Err(Error::Io(_))
        );
    }

    // --------------------------------------------------------------
    // Equivalence types
    // --------------------------------------------------------------
    #[test]
    // char -> str
    fn test_deserializer_deserialize_char() {
        let data = [1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99];
        let mut deserializer = super::Deserializer::from_slice(&data);
        // `deserialize_char` yields strings, the visitor decides if it handles them or not.
        assert_matches!(
            deserializer.deserialize_char(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "a")
        );
        assert_matches!(
            deserializer.deserialize_char(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "bc")
        );
        assert_matches!(
            deserializer.deserialize_char(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    // str -> raw
    fn test_deserializer_deserialize_str() {
        let data = [1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 3, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_str(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "a")
        );
        assert_matches!(
            deserializer.deserialize_str(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "bc")
        );
        assert_matches!(
            deserializer.deserialize_str(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    // string -> raw
    fn test_deserializer_deserialize_string() {
        let data = [1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 0, 0, 0, 0, 3, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_string(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "a")
        );
        assert_matches!(
            deserializer.deserialize_string(ValueVisitor),
            Ok(Value::String(s)) => assert_eq!(s, "bc")
        );
        assert_matches!(
            deserializer.deserialize_string(ValueVisitor),
            Ok(Value::String(s)) => assert!(s.is_empty())
        );
        assert_matches!(
            deserializer.deserialize_string(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_byte_buf() {
        let data = [1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 0, 0, 0, 0, 3, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert_eq!(b, [97])
        );
        assert_matches!(
            deserializer.deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert_eq!(b, [98, 99])
        );
        assert_matches!(
            deserializer.deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert!(b.is_empty())
        );
        assert_matches!(
            deserializer.deserialize_byte_buf(ValueVisitor),
            Err(Error::Io(_))
        );
    }

    #[test]
    // struct(T...) -> tuple(T...)
    fn test_deserializer_deserialize_struct() {
        let data = [1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 0, 0, 0, 0, 3, 0, 0, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S {
            c: char,
            s: std::string::String,
            t: (u8, i8, i16),
            i: i32,
        }
        assert_matches!(
            S::deserialize(&mut deserializer),
            Ok(S {
                c: 'a',
                s,
                t: (0, 0, 0),
                i: 3
            }) => assert_eq!(s, "bc")
        );
        assert_matches!(S::deserialize(&mut deserializer), Err(Error::Io(_)));
    }

    #[test]
    // newtype_struct(T) -> tuple(T) = T
    fn test_deserializer_deserialize_newtype_struct() {
        let data = [1, 0, 0, 0, 97, 1, 0, 0, 0, 98];
        let mut deserializer = super::Deserializer::from_slice(&data);
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S(char);
        assert_matches!(S::deserialize(&mut deserializer), Ok(S('a')));
        assert_matches!(S::deserialize(&mut deserializer), Ok(S('b')));
        assert_matches!(S::deserialize(&mut deserializer), Err(Error::Io(_)));
    }

    #[test]
    // unit_struct -> unit
    fn test_deserializer_deserialize_unit_struct() {
        let data = [];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_unit_struct("MyStruct", ValueVisitor),
            Ok(Value::Unit)
        );
    }

    #[test]
    // tuple_struct(T...) -> tuple(T...)
    fn test_deserializer_deserialize_tuple_struct() {
        let data = [1, 0, 0, 0, 97, 3, 0, 0, 0, 4, 0, 5, 0, 6, 0];
        let mut deserializer = super::Deserializer::from_slice(&data);
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S<'s>(&'s str, Vec<i16>);
        assert_matches!(S::deserialize(&mut deserializer), Ok(S("a", v)) => assert_eq!(v, [4, 5, 6]));
    }

    #[test]
    // enum(idx,T) -> tuple(idx,T)
    fn test_deserializer_deserialize_enum() {
        let data = [
            0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 97, 3, 0, 0, 0, 4, 0, 5, 0, 6, 0,
        ];
        let mut deserializer = super::Deserializer::from_slice(&data);
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        enum E {
            A,
            B(char),
            C,
            D(i16, i16, i16),
        }
        assert_matches!(E::deserialize(&mut deserializer), Ok(E::A));
        assert_matches!(E::deserialize(&mut deserializer), Ok(E::B('a')));
        assert_matches!(E::deserialize(&mut deserializer), Ok(E::D(4, 5, 6)));
        assert_matches!(E::deserialize(&mut deserializer), Err(Error::Io(_)));
    }

    #[test]
    // identifier => unit
    fn test_deserializer_deserialize_identifier() {
        let data = [];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_identifier(ValueVisitor),
            Ok(Value::Unit)
        );
    }

    // --------------------------------------------------------------
    // Unhandled types
    // --------------------------------------------------------------
    #[test]
    fn test_deserializer_deserialize_i128() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_i128(ValueVisitor),
            Err(Error::Custom(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_u128() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_u128(ValueVisitor),
            Err(Error::Custom(_))
        );
    }

    #[test]
    fn test_deserializer_deserialize_any() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_any(ValueVisitor),
            Err(Error::CannotDeserializeAny)
        );
    }

    #[test]
    fn test_deserializer_deserialize_ignored_any() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut deserializer = super::Deserializer::from_slice(&data);
        assert_matches!(
            deserializer.deserialize_any(ValueVisitor),
            Err(Error::CannotDeserializeAny)
        );
    }
}
