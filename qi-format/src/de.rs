use crate::{read, Error, Raw, Result, String};

pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: std::io::Read,
    T: serde::de::DeserializeOwned,
{
    let mut de = Deserializer::from_reader(reader);
    T::deserialize(&mut de)
}

pub fn from_bytes<'b, T>(bytes: &'b [u8]) -> Result<T>
where
    T: serde::de::Deserialize<'b>,
{
    let mut de = Deserializer::from_slice(bytes);
    T::deserialize(&mut de)
}

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Deserializer<R> {
    reader: R,
}

impl<R> Deserializer<read::IoRead<R>>
where
    R: std::io::Read,
{
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader: read::IoRead::new(reader),
        }
    }
}

impl<'b> Deserializer<read::SliceRead<'b>> {
    pub fn from_slice(data: &'b [u8]) -> Self {
        Self {
            reader: read::SliceRead::new(data),
        }
    }
}

impl<'de, R> serde::Deserializer<'de> for &mut Deserializer<R>
where
    R: read::Read<String = String<'de>, Raw = Raw<'de>>,
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

    // equivalence char -> string
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
        let str = self.reader.read_string()?;
        str.deserialize_str(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let str = self.reader.read_string()?;
        str.deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let raw = self.reader.read_raw()?;
        raw.deserialize_bytes(visitor)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
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

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
        //let size = self.readerread_size(&mut self.reader)?;
        //let deserializer = self.deserialize_tuple(size);
        //visitor.visit_seq(deserializer)
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
        //let deserializer = self.deserialize_tuple(len);
        //visitor.visit_seq(deserializer)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
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
{
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, _seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        //use serde::de::IntoDeserializer;
        //let variant_index = read_u32(&mut self.reader)?;
        //let variant: Result<_> = seed.deserialize(variant_index.into_deserializer());
        //Ok((variant?, self))
        todo!()
    }
}

impl<'de, R> serde::de::VariantAccess<'de> for &mut Deserializer<R>
where
    R: read::Read,
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        todo!()
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        todo!()
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }
}

struct DeserializeTuple<R> {
    iter: std::ops::Range<usize>,
    reader: R,
}

impl<R> DeserializeTuple<R>
where
    R: read::Read,
{
    fn from_size_and_deserializer(size: usize, reader: R) -> Self {
        Self {
            iter: 0..size,
            reader,
        }
    }

    fn next_item<'de, T>(&mut self, _seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        todo!()
        //self.iter
        //    .next()
        //    .map(move |_idx| -> Result<T::Value> {
        //        from_reader_and_seed(self.reader.by_ref(), seed)
        //    })
        //    .transpose()
    }
}

impl<'de, R> serde::de::SeqAccess<'de> for DeserializeTuple<R>
where
    R: read::Read,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.next_item(seed)
    }
}

impl<'de, R> serde::de::MapAccess<'de> for DeserializeTuple<R>
where
    R: read::Read,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        self.next_item(seed)
    }

    fn next_value_seed<V>(&mut self, _seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        todo!()
        //from_reader_and_seed(self.reader.by_ref(), seed)
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
    // --------------------------------------------------------------
    // Serde types
    // --------------------------------------------------------------

    // --------------------------------------------------------------
    // Equivalence types
    // --------------------------------------------------------------
}
