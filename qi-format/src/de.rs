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
use serde::de::IntoDeserializer;

pub fn from_slice<'b, T>(data: &'b [u8]) -> Result<T>
where
    T: serde::Deserialize<'b>,
{
    T::deserialize(&mut SliceDeserializer::new(data))
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SliceDeserializer<'data> {
    data: &'data [u8],
}

impl<'data> SliceDeserializer<'data> {
    pub fn new(bytes: &'data [u8]) -> Self {
        Self { data: bytes }
    }

    pub fn by_ref(&mut self) -> &mut Self {
        self
    }
}

impl<'data> serde::Deserializer<'data> for &mut SliceDeserializer<'data> {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        Err(Error::UnknownElement)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_bool(read::read_bool(&mut self.data)?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_i8(read::read_i8(&mut self.data)?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_i16(read::read_i16(&mut self.data)?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_i32(read::read_i32(&mut self.data)?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_i64(read::read_i64(&mut self.data)?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_u8(read::read_u8(&mut self.data)?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_u16(read::read_u16(&mut self.data)?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_u32(read::read_u32(&mut self.data)?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_u64(read::read_u64(&mut self.data)?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_f32(read::read_f32(&mut self.data)?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_f64(read::read_f64(&mut self.data)?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        let str = read::read_str(&mut self.data)?;
        let c = str.chars().next().ok_or(Error::ShortRead)?;
        visitor.visit_char(c)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_borrowed_str(read::read_str(&mut self.data)?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_string(read::read_string(&mut self.data)?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_borrowed_bytes(read::read_raw(&mut self.data)?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_byte_buf(read::read_raw_buf(&mut self.data)?.to_vec())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        match read::read_bool(&mut self.data)? {
            true => visitor.visit_some(self),
            false => visitor.visit_none(),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        // nothing
        visitor.visit_unit()
    }

    // equivalence: unit_struct -> unit
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        self.deserialize_unit(visitor)
    }

    // equivalence: newtype_struct(T) = tuple(T) = T
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        let access = SequenceAccess::new_size_prefixed(self)?;
        visitor.visit_seq(access)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_seq(SequenceAccess::new(len, self, None, None))
    }

    // equivalence: tuple_struct(T...) -> tuple(T...)
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        let access = SequenceAccess::new_size_prefixed(self)?;
        visitor.visit_map(access)
    }

    // equivalence: struct(T...) -> tuple(T...)
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_seq(SequenceAccess::new(
            fields.len(),
            self,
            Some(name),
            Some(fields),
        ))
    }

    // equivalence: enum(idx,T) -> tuple(idx,T)
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        visitor.visit_enum(self)
    }

    // equivalence: identifier -> unit
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        Err(Error::UnknownElement)
    }

    fn deserialize_i128<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        let _ = visitor;
        Err(serde::de::Error::custom("i128 is not supported"))
    }

    fn deserialize_u128<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        let _ = visitor;
        Err(serde::de::Error::custom("u128 is not supported"))
    }
}

impl<'data> serde::de::EnumAccess<'data> for &mut SliceDeserializer<'data> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'data>,
    {
        let variant_index =
            read::read_u32(&mut self.data).map_err(|err| Error::VariantIndex(err.into()))?;
        let variant_index_deserializer = variant_index.into_deserializer();
        let value: Result<_> = seed.deserialize(variant_index_deserializer);
        Ok((value?, self))
    }
}

impl<'data> serde::de::VariantAccess<'data> for &mut SliceDeserializer<'data> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: serde::de::DeserializeSeed<'data>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        use serde::Deserializer;
        self.deserialize_tuple(len, visitor)
    }

    // equivalence: struct(T...) -> tuple(T...)
    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        use serde::Deserializer;
        self.deserialize_struct("", fields, visitor)
    }
}

struct SequenceAccess<'slice_de, 'data> {
    index: usize,
    size: usize,
    deserializer: &'slice_de mut SliceDeserializer<'data>,
    name: Option<&'static str>,
    fields: Option<&'static [&'static str]>,
}

impl<'slice_de, 'data> SequenceAccess<'slice_de, 'data> {
    fn new_size_prefixed(deserializer: &'slice_de mut SliceDeserializer<'data>) -> Result<Self> {
        let size = read::read_size(&mut deserializer.data)
            .map_err(|err| Error::SequenceSize(Box::new(err)))?;
        Ok(Self::new(size, deserializer, None, None))
    }

    fn new(
        size: usize,
        deserializer: &'slice_de mut SliceDeserializer<'data>,
        name: Option<&'static str>,
        fields: Option<&'static [&'static str]>,
    ) -> Self {
        Self {
            index: 0,
            size,
            deserializer,
            name,
            fields,
        }
    }

    fn next_item<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'data>,
    {
        if self.index < self.size {
            let item = seed.deserialize(self.deserializer.by_ref())?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }
}

impl<'slice_de, 'data> serde::de::SeqAccess<'data> for SequenceAccess<'slice_de, 'data> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'data>,
    {
        let item = self.next_item(seed).map_err(|err| Error::SequenceElement {
            name: format!(
                "{}{}",
                match self.name {
                    Some(name) => format!("{name}."),
                    None => String::new(),
                },
                match self.fields {
                    Some(fields) => fields[self.index].to_owned(),
                    None => self.index.to_string(),
                }
            ),
            source: Box::new(err),
        })?;
        self.index += 1;
        Ok(item)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.size.saturating_sub(self.index))
    }
}

impl<'slice_de, 'data> serde::de::MapAccess<'data> for SequenceAccess<'slice_de, 'data> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'data>,
    {
        self.next_item(seed).map_err(|err| Error::MapKey {
            index: self.index,
            source: err.into(),
        })
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'data>,
    {
        let item = seed
            .deserialize(self.deserializer.by_ref())
            .map_err(|err| Error::MapValue {
                index: self.index,
                source: err.into(),
            })?;
        self.index += 1;
        Ok(item)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.size.saturating_sub(self.index))
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
    use super::SliceDeserializer as D;
    use super::*;
    use assert_matches::assert_matches;
    use serde::de::Deserializer;
    use serde_value::{Value, ValueVisitor};

    #[test]
    fn test_slice_deserializer_deserialize_bool() {
        assert_matches!(from_slice::<bool>(&[0]), Ok(false));
        assert_matches!(from_slice::<bool>(&[1]), Ok(true));
        assert_matches!(from_slice::<bool>(&[2]), Err(Error::NotABoolValue(2)));
        assert_matches!(from_slice::<bool>(&[]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_i8() {
        assert_matches!(from_slice::<i8>(&[1]), Ok(1));
        assert_matches!(from_slice::<i8>(&[2]), Ok(2));
        assert_matches!(from_slice::<i8>(&[]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_u8() {
        assert_matches!(from_slice::<u8>(&[1]), Ok(1));
        assert_matches!(from_slice::<u8>(&[2]), Ok(2));
        assert_matches!(from_slice::<u8>(&[]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_i16() {
        assert_matches!(from_slice::<i16>(&[1, 0]), Ok(1));
        assert_matches!(from_slice::<i16>(&[2, 0]), Ok(2));
        assert_matches!(from_slice::<i16>(&[]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_u16() {
        assert_matches!(from_slice::<u16>(&[1, 0]), Ok(1));
        assert_matches!(from_slice::<u16>(&[2, 0]), Ok(2));
        assert_matches!(from_slice::<u16>(&[0]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_i32() {
        assert_matches!(from_slice::<i32>(&[1, 0, 0, 0]), Ok(1));
        assert_matches!(from_slice::<i32>(&[2, 0, 0, 0]), Ok(2));
        assert_matches!(from_slice::<i32>(&[0, 0, 0]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_u32() {
        assert_matches!(from_slice::<u32>(&[1, 0, 0, 0]), Ok(1));
        assert_matches!(from_slice::<u32>(&[2, 0, 0, 0]), Ok(2));
        assert_matches!(from_slice::<u32>(&[0, 0, 0]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_i64() {
        assert_matches!(from_slice::<i64>(&[1, 0, 0, 0, 0, 0, 0, 0]), Ok(1));
        assert_matches!(from_slice::<i64>(&[2, 0, 0, 0, 0, 0, 0, 0]), Ok(2));
        assert_matches!(
            from_slice::<i64>(&[0, 0, 0, 0, 0, 0, 0]),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_u64() {
        assert_matches!(from_slice::<u64>(&[1, 0, 0, 0, 0, 0, 0, 0]), Ok(1));
        assert_matches!(from_slice::<u64>(&[2, 0, 0, 0, 0, 0, 0, 0]), Ok(2));
        assert_matches!(
            from_slice::<u64>(&[0, 0, 0, 0, 0, 0, 0]),
            Err(Error::ShortRead)
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_slice_deserializer_deserialize_f32() {
        assert_matches!(from_slice::<f32>(&[0x14, 0xae, 0x29, 0x42]), Ok(42.42));
        assert_matches!(
            from_slice::<f32>(&[0xff, 0xff, 0xff, 0x7f]),
            Ok(f) => assert!(f.is_nan())
        );
        assert_matches!(from_slice::<f32>(&[0, 0, 0]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_f64() {
        assert_matches!(
            from_slice::<f64>(&[0xf6, 0x28, 0x5c, 0x8f, 0xc2, 0x35, 0x45, 0x40]),
            Ok(42.42)
        );
        assert_matches!(
            from_slice::<f64>(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]),
            Ok(f) => assert!(f.is_nan())
        );
        assert_matches!(
            from_slice::<f64>(&[0, 0, 0, 0, 0, 0, 0]),
            Err(Error::ShortRead)
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_bytes() {
        assert_matches!(
            D::new(&[4, 0, 0, 0, 1, 2, 3, 4]).deserialize_bytes(ValueVisitor),
            Ok(Value::Bytes(v)) => assert_eq!(v, [1, 2, 3, 4])
        );
        assert_matches!(
            D::new(&[0, 0, 0, 0]).deserialize_bytes(ValueVisitor),
            Ok(Value::Bytes(v)) => assert!(v.is_empty())
        );
        assert_matches!(
            D::new(&[4, 0, 0, 0, 1, 2, 3]).deserialize_bytes(ValueVisitor),
            Err(Error::ShortRead)
        );
        assert_matches!(
            D::new(&[0, 0, 0]).deserialize_bytes(ValueVisitor),
            Err(Error::SequenceSize(_))
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_option() {
        assert_matches!(from_slice::<Option::<i16>>(&[1, 42, 0]), Ok(Some(42i16)));
        assert_matches!(from_slice::<Option::<i16>>(&[0]), Ok(None));
        assert_matches!(from_slice::<Option::<i16>>(&[]), Err(Error::ShortRead));
    }

    #[test]
    fn test_slice_deserializer_deserialize_unit() {
        assert_matches!(from_slice::<()>(&[]), Ok(()));
    }

    #[test]
    fn test_slice_deserializer_deserialize_sequence() {
        assert_matches!(
            from_slice::<Vec::<i16>>(&[3, 0, 0, 0, 1, 0, 2, 0, 3, 0]),
            Ok(v) => assert_eq!(v, [1, 2, 3])
        );
        assert_matches!(
            from_slice::<Vec::<i16>>(&[0, 0, 0, 0]),
            Ok(v) => assert!(v.is_empty())
        );
        assert_matches!(
            from_slice::<Vec::<i16>>(&[1, 0, 0, 0]),
            Err(Error::SequenceElement { .. })
        );
        assert_matches!(
            from_slice::<Vec::<i16>>(&[0, 0, 0]),
            Err(Error::SequenceSize(_))
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_tuple() {
        assert_matches!(
            from_slice::<(u32, Option<i8>)>(&[2, 0, 0, 0, 1, 2]),
            Ok((2, Some(2)))
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_map() {
        use std::collections::HashMap;
        assert_matches!(
            from_slice::<HashMap::<i8, u8>>(&[2, 0, 0, 0, 1, 2, 2, 4]),
            Ok(m) => assert_eq!(m, HashMap::from([(1, 2), (2, 4)]))
        );
        assert_matches!(
            from_slice::<HashMap::<i8, u8>>(&[1, 0, 0, 0, 0]),
            Err(Error::MapValue { index: 0, .. })
        );
        assert_matches!(
            from_slice::<HashMap::<i8, u8>>(&[1, 0, 0, 0]),
            Err(Error::MapKey { index: 0, .. })
        );
        assert_matches!(
            from_slice::<HashMap::<i8, u8>>(&[0, 0, 0]),
            Err(Error::SequenceSize(_))
        );
    }

    // --------------------------------------------------------------
    // Equivalence types
    // --------------------------------------------------------------
    #[test]
    // char -> str
    fn test_slice_deserializer_deserialize_char() {
        // `deserialize_char` yields strings, the visitor decides if it handles them or not.
        assert_matches!(from_slice::<char>(&[1, 0, 0, 0, 97]), Ok('a'));
        assert_matches!(from_slice::<char>(&[2, 0, 0, 0, 98, 99]), Ok('b'));
        assert_matches!(from_slice::<char>(&[1, 0, 0, 0]), Err(Error::ShortRead));
        assert_matches!(from_slice::<char>(&[0, 0, 0]), Err(Error::SequenceSize(_)));
    }

    #[test]
    // str -> raw
    fn test_slice_deserializer_deserialize_str() {
        assert_matches!(
            from_slice::<&str>(&[1, 0, 0, 0, 97]),
            Ok(s) => assert_eq!(s, "a")
        );
        assert_matches!(
            from_slice::<&str>(&[2, 0, 0, 0, 98, 99]),
            Ok(s) => assert_eq!(s, "bc")
        );
        assert_matches!(from_slice::<&str>(&[1, 0, 0, 0]), Err(Error::ShortRead));
        assert_matches!(from_slice::<&str>(&[0, 0, 0]), Err(Error::SequenceSize(_)));
    }

    #[test]
    fn test_slice_deserializer_deserialize_string() {
        assert_matches!(
            from_slice::<String>(&[1, 0, 0, 0, 97]),
            Ok(s) => assert_eq!(s, "a")
        );
        assert_matches!(
            from_slice::<String>(&[2, 0, 0, 0, 98, 99]),
            Ok(s) => assert_eq!(s, "bc")
        );
        assert_matches!(
            from_slice::<String>(&[0, 0, 0, 0]),
            Ok(s) => assert!(s.is_empty())
        );
        assert_matches!(from_slice::<String>(&[1, 0, 0, 0]), Err(Error::ShortRead));
        assert_matches!(
            from_slice::<String>(&[0, 0, 0]),
            Err(Error::SequenceSize(_))
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_byte_buf() {
        assert_matches!(
            D::new(&[1, 0, 0, 0, 97]).deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert_eq!(b, [97])
        );
        assert_matches!(
            D::new(&[2, 0, 0, 0, 98, 99]).deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert_eq!(b, [98, 99])
        );
        assert_matches!(
            D::new(&[0, 0, 0, 0]).deserialize_byte_buf(ValueVisitor),
            Ok(Value::Bytes(b)) => assert!(b.is_empty())
        );
        assert_matches!(
            D::new(&[1, 0, 0, 0]).deserialize_byte_buf(ValueVisitor),
            Err(Error::ShortRead)
        );
        assert_matches!(
            D::new(&[0, 0, 0]).deserialize_byte_buf(ValueVisitor),
            Err(Error::SequenceSize(_))
        );
    }

    #[test]
    // struct(T...) -> tuple(T...)
    fn test_slice_deserializer_deserialize_struct() {
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S {
            c: char,
            s: std::string::String,
            // t: (u8, i8, i16),
            // i: i32,
        }
        assert_matches!(
            from_slice::<S>(&[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99]),
            Ok(S { c: 'a', s}) => assert_eq!(s, "bc")
        );
        // assert_matches!(
        //     from_slice::<S>(&[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 0, 0, 0, 0, 3, 0, 0, 0]),
        //     Ok(S {
        //         c: 'a',
        //         s,
        //         t: (0, 0, 0),
        //         i: 3
        //     }) => assert_eq!(s, "bc")
        // );
    }

    #[test]
    // newtype_struct(T) -> tuple(T) = T
    fn test_slice_deserializer_deserialize_newtype_struct() {
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S(char);
        assert_matches!(from_slice::<S>(&[1, 0, 0, 0, 97]), Ok(S('a')));
        assert_matches!(from_slice::<S>(&[1, 0, 0, 0, 98]), Ok(S('b')));
    }

    #[test]
    // unit_struct -> unit
    fn test_slice_deserializer_deserialize_unit_struct() {
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S;
        assert_matches!(from_slice::<S>(&[]), Ok(S));
    }

    #[test]
    // tuple_struct(T...) -> tuple(T...)
    fn test_slice_deserializer_deserialize_tuple_struct() {
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        struct S(String, Vec<i16>);
        assert_matches!(
            from_slice::<S>(&[1, 0, 0, 0, 97, 3, 0, 0, 0, 4, 0, 5, 0, 6, 0]),
            Ok(S(str, v)) => {
                assert_eq!(str, "a");
                assert_eq!(v, [4, 5, 6])
            }
        );
    }

    #[test]
    // enum(idx,T) -> tuple(idx,T)
    fn test_slice_deserializer_deserialize_enum() {
        #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
        enum E {
            A,
            B(char),
            C,
            D(i16, i16, i16),
        }
        assert_matches!(from_slice::<E>(&[0, 0, 0, 0]), Ok(E::A));
        assert_matches!(
            from_slice::<E>(&[1, 0, 0, 0, 1, 0, 0, 0, 97]),
            Ok(E::B('a'))
        );
        assert_matches!(
            from_slice::<E>(&[3, 0, 0, 0, 4, 0, 5, 0, 6, 0]),
            Ok(E::D(4, 5, 6))
        );
    }

    #[test]
    // identifier => unit
    fn test_slice_deserializer_deserialize_identifier() {
        assert_matches!(
            D::new(&[]).deserialize_identifier(ValueVisitor),
            Ok(Value::Unit)
        );
    }

    // --------------------------------------------------------------
    // Unhandled types
    // --------------------------------------------------------------
    #[test]
    fn test_slice_deserializer_deserialize_i128() {
        assert_matches!(
            D::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
                .deserialize_i128(ValueVisitor),
            Err(Error::Custom(_))
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_u128() {
        assert_matches!(
            D::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
                .deserialize_u128(ValueVisitor),
            Err(Error::Custom(_))
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_any() {
        assert_matches!(
            D::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
                .deserialize_any(ValueVisitor),
            Err(Error::UnknownElement)
        );
    }

    #[test]
    fn test_slice_deserializer_deserialize_ignored_any() {
        assert_matches!(
            D::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
                .deserialize_any(ValueVisitor),
            Err(Error::UnknownElement)
        );
    }
}
