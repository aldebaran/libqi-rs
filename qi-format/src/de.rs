use crate::{
    num_bool::{FALSE_BOOL, TRUE_BOOL},
    Error, Result,
};
use std::convert::TryInto;

pub fn from_reader<'de, R, T>(reader: R) -> Result<T>
where
    R: std::io::Read,
    T: serde::de::Deserialize<'de>,
{
    T::deserialize(&mut Deserializer::from_reader(reader))
}

pub fn from_bytes<'b, T>(bytes: &'b [u8]) -> Result<T>
where
    T: serde::de::Deserialize<'b>,
{
    // TODO: BytesDeserializer to avoid copying data.
    from_reader(bytes)
}

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Deserializer<R> {
    reader: R,
}

impl<R> Deserializer<R>
where
    R: std::io::Read,
{
    pub fn from_reader(reader: R) -> Self {
        Self { reader }
    }

    fn deserialize_tuple(&mut self, len: usize) -> DeserializeTuple<&mut R> {
        DeserializeTuple::from_size_and_deserializer(len, self.reader.by_ref())
    }

    fn by_ref(&mut self) -> &mut Self {
        self
    }
}

impl<'de, R> serde::Deserializer<'de> for &mut Deserializer<R>
where
    R: std::io::Read,
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
        visitor.visit_bool(read_bool(&mut self.reader)?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let byte = read_byte(&mut self.reader)?;
        visitor.visit_i8(byte as i8)
    }

    // LibQi does not handle endianness correctly, and as such always
    // deserialize integers with native byte order. However, as it mostly
    // executes on little endian systems, we assume they are always
    // encoded as such, to ensure portability with systems that are not
    // little endian.

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let bytes = read_bytes(&mut self.reader)?;
        visitor.visit_i16(i16::from_le_bytes(bytes))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let bytes = read_bytes(&mut self.reader)?;
        visitor.visit_i32(i32::from_le_bytes(bytes))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let bytes = read_bytes(&mut self.reader)?;
        visitor.visit_i64(i64::from_le_bytes(bytes))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u8(read_byte(&mut self.reader)?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let bytes = read_bytes(&mut self.reader)?;
        visitor.visit_u16(u16::from_le_bytes(bytes))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u32(read_u32(&mut self.reader)?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let bytes = read_bytes(&mut self.reader)?;
        visitor.visit_u64(u64::from_le_bytes(bytes))
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let bytes = read_bytes(&mut self.reader)?;
        visitor.visit_f32(f32::from_le_bytes(bytes))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let bytes = read_bytes(&mut self.reader)?;
        visitor.visit_f64(f64::from_le_bytes(bytes))
    }

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
        let buf = read_bytes_seq(&mut self.reader)?;
        let str = std::str::from_utf8(&buf)?;
        visitor.visit_str(str)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let buf = read_bytes_seq(&mut self.reader)?;
        let str = String::from_utf8(buf).map_err(|e| e.utf8_error())?;
        visitor.visit_string(str)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let buf = read_bytes_seq(&mut self.reader)?;
        visitor.visit_bytes(&buf)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let buf = read_bytes_seq(&mut self.reader)?;
        visitor.visit_byte_buf(buf)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match read_bool(&mut self.reader)? {
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

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let size = read_size(&mut self.reader)?;
        let deserializer = self.deserialize_tuple(size);
        visitor.visit_seq(deserializer)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let deserializer = self.deserialize_tuple(len);
        visitor.visit_seq(deserializer)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let deserializer = self.deserialize_tuple(len);
        visitor.visit_seq(deserializer)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let size = read_size(&mut self.reader)?;
        let deserializer = self.deserialize_tuple(size);
        visitor.visit_map(deserializer)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let deserializer = self.deserialize_tuple(fields.len());
        visitor.visit_seq(deserializer)
    }

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

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de, R> serde::de::EnumAccess<'de> for &mut Deserializer<R>
where
    R: std::io::Read,
{
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        use serde::de::IntoDeserializer;
        let variant_index = read_u32(&mut self.reader)?;
        let variant: Result<_> = seed.deserialize(variant_index.into_deserializer());
        Ok((variant?, self))
    }
}

impl<'de, R> serde::de::VariantAccess<'de> for &mut Deserializer<R>
where
    R: std::io::Read,
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        serde::Deserialize::deserialize(self)
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
        let deserializer = self.deserialize_tuple(len);
        visitor.visit_seq(deserializer)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let deserializer = self.deserialize_tuple(fields.len());
        visitor.visit_seq(deserializer)
    }
}

struct DeserializeTuple<R> {
    iter: std::ops::Range<usize>,
    reader: R,
}

impl<R> DeserializeTuple<R>
where
    R: std::io::Read,
{
    fn from_size_and_deserializer(size: usize, reader: R) -> Self {
        Self {
            iter: 0..size,
            reader,
        }
    }

    fn next_item<'de, T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.iter
            .next()
            .map(move |_idx| -> Result<T::Value> {
                from_reader_and_seed(self.reader.by_ref(), seed)
            })
            .transpose()
    }
}

impl<'de, R> serde::de::SeqAccess<'de> for DeserializeTuple<R>
where
    R: std::io::Read,
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
    R: std::io::Read,
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
        from_reader_and_seed(self.reader.by_ref(), seed)
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

fn read_byte<R>(reader: &mut R) -> std::io::Result<u8>
where
    R: std::io::Read,
{
    let mut buf = [0; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_bytes<R, const N: usize>(reader: &mut R) -> std::io::Result<[u8; N]>
where
    R: std::io::Read,
{
    let mut buf = [0; N];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

fn read_bool<R>(reader: &mut R) -> Result<bool>
where
    R: std::io::Read,
{
    let byte = read_byte(reader)?;
    match byte {
        FALSE_BOOL => Ok(false),
        TRUE_BOOL => Ok(true),
        _ => Err(Error::NotABoolValue(byte)),
    }
}

fn read_u32<R>(reader: &mut R) -> std::io::Result<u32>
where
    R: std::io::Read,
{
    let bytes = read_bytes(reader)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_size<R>(reader: &mut R) -> Result<usize>
where
    R: std::io::Read,
{
    let size_bytes = read_bytes(reader)?;
    let size = u32::from_le_bytes(size_bytes)
        .try_into()
        .map_err(Error::BadSize)?;
    Ok(size)
}

fn read_bytes_seq<R>(reader: &mut R) -> Result<Vec<u8>>
where
    R: std::io::Read,
{
    let size = read_size(reader)?;
    let mut buf = vec![0; size];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

fn from_reader_and_seed<'de, R, T>(reader: R, seed: T) -> Result<T::Value>
where
    R: std::io::Read,
    T: serde::de::DeserializeSeed<'de>,
{
    seed.deserialize(&mut Deserializer::from_reader(reader))
}

#[cfg(test)]
mod tests {
    // ! TODO !

    #[test]
    fn test_deserializer_error_rewind_reader() {
        todo!()
    }
}
