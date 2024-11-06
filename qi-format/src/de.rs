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
        match read::read_str(&mut self.data)? {
            read::StrOrBytes::Str(str) => {
                let c = str.chars().next().ok_or(Error::ShortRead)?;
                visitor.visit_char(c)
            }
            read::StrOrBytes::Bytes(bytes) => {
                let first = bytes.iter().next().ok_or(Error::ShortRead)?;
                visitor.visit_u8(*first)
            }
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        match read::read_str(&mut self.data)? {
            read::StrOrBytes::Str(str) => visitor.visit_borrowed_str(str),
            read::StrOrBytes::Bytes(bytes) => visitor.visit_borrowed_bytes(bytes),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'data>,
    {
        match read::read_string(&mut self.data)? {
            read::StringOrByteBuf::String(str) => visitor.visit_string(str),
            read::StringOrByteBuf::ByteBuf(bytes) => visitor.visit_byte_buf(bytes),
        }
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

impl<'data> serde::Deserializer<'data> for SliceDeserializer<'data> {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_any(visitor)
    }

    fn deserialize_bool<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_bool(visitor)
    }

    fn deserialize_i8<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_i8(visitor)
    }

    fn deserialize_i16<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_i16(visitor)
    }

    fn deserialize_i32<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_i32(visitor)
    }

    fn deserialize_i64<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_i64(visitor)
    }

    fn deserialize_u8<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_u8(visitor)
    }

    fn deserialize_u16<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_u8(visitor)
    }

    fn deserialize_u32<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_u32(visitor)
    }

    fn deserialize_u64<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_u64(visitor)
    }

    fn deserialize_f32<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_f32(visitor)
    }

    fn deserialize_f64<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_f64(visitor)
    }

    fn deserialize_char<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_char(visitor)
    }

    fn deserialize_str<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_str(visitor)
    }

    fn deserialize_string<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_bytes(visitor)
    }

    fn deserialize_byte_buf<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_byte_buf(visitor)
    }

    fn deserialize_option<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_option(visitor)
    }

    fn deserialize_unit<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_unit(visitor)
    }

    fn deserialize_unit_struct<V>(
        mut self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_unit_struct(name, visitor)
    }

    fn deserialize_newtype_struct<V>(
        mut self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_seq(visitor)
    }

    fn deserialize_tuple<V>(
        mut self,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple_struct<V>(
        mut self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_tuple_struct(name, len, visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_map(visitor)
    }

    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V>(
        mut self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_enum(name, variants, visitor)
    }

    fn deserialize_identifier<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_identifier(visitor)
    }

    fn deserialize_ignored_any<V>(
        mut self,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'data>,
    {
        self.by_ref().deserialize_ignored_any(visitor)
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
