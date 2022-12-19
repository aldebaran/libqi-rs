use crate::{
    num_bool::{FALSE_BOOL, TRUE_BOOL},
    Error, Raw, Result, String,
};
use serde::Serialize;

pub fn to_writer<W, T>(writer: W, value: &T) -> Result<W>
where
    W: std::io::Write,
    T: ?Sized + Serialize,
{
    let mut serializer = Serializer::from_writer(writer);
    value.serialize(&mut serializer)?;
    Ok(serializer.writer)
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut buf = Vec::new();
    to_writer(&mut buf, value)?;
    Ok(buf)
}

pub fn write_byte<W>(writer: W, b: u8) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_word<W>(writer: W, w: [u8; 2]) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_dword<W>(writer: W, dw: [u8; 4]) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_qword<W>(writer: W, qw: [u8; 8]) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_bool<W>(writer: W, val: bool) -> Result<()>
where
    W: std::io::Write,
{
    write_byte(writer, if val { TRUE_BOOL } else { FALSE_BOOL })
}

// LibQi does not handle endianness correctly, and as such always
// serialize integers with native byte order. However, as it mostly
// executes on little endian systems, we assume they are always
// encoded as such, to ensure portability with systems that are not
// little endian.

pub fn write_u8<W>(writer: W, val: u8) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_i8<W>(writer: W, val: i8) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_u16<W>(mut writer: W, val: u16) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(&val.to_le_bytes())?;
    Ok(())
}

pub fn write_i16<W>(writer: W, val: i16) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_u32<W>(mut writer: W, val: u32) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(&val.to_le_bytes())?;
    Ok(())
}

pub fn write_i32<W>(writer: W, val: i32) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_u64<W>(mut writer: W, val: u64) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(&val.to_le_bytes())?;
    Ok(())
}

pub fn write_i64<W>(writer: W, val: i64) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_size<W>(writer: W, size: usize) -> Result<()>
where
    W: std::io::Write,
{
    let size = std::convert::TryFrom::try_from(size).map_err(Error::BadSize)?;
    write_u32(writer, size)
}

pub fn write_string<W>(writer: W, str: &String) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

pub fn write_raw<W>(writer: W, raw: &Raw) -> Result<()>
where
    W: std::io::Write,
{
    todo!()
}

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Serializer<W> {
    writer: W,
}

impl<W> Serializer<W>
where
    W: std::io::Write,
{
    pub fn from_writer(writer: W) -> Self {
        Self { writer }
    }

    fn by_ref(&mut self) -> &mut Self {
        self
    }
}

impl<'s, W> serde::Serializer for &'s mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        write_bool(self.writer.by_ref(), v)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.serialize_u8(v as u8)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.writer.write_all(&[v])?;
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.serialize_u16(v as u16)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.writer.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.serialize_u32(v as u32)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        write_u32(self.writer.by_ref(), v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        self.writer.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.writer.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.writer.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let mut buf = [0; 4];
        let s = v.encode_utf8(&mut buf);
        self.serialize_str(s)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.collect_seq(v)
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.serialize_bool(false)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        self.serialize_bool(true)?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        // nothing
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_u32(variant_index)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        self.serialize_u32(variant_index)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let size = len.ok_or(Error::ExpectedListSize)?;
        let size = write_size(self.writer.by_ref(), size)?;
        size.serialize(self.by_ref())?;
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.serialize_u32(variant_index)?;
        self.serialize_tuple(len)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        self.serialize_seq(len)
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.serialize_tuple_variant(name, variant_index, variant, len)
    }
}

impl<W> serde::ser::SerializeSeq for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self.by_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeTuple for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self.by_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeTupleStruct for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self.by_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeTupleVariant for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self.by_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeMap for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize,
    {
        key.serialize(self.by_ref())?;
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self.by_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeStruct for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self.by_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeStructVariant for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self.by_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl serde::ser::Error for Error {
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

    // --------------------------------------------------------------
    // Qi simple types
    // --------------------------------------------------------------

    #[test]
    fn test_write_byte() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_word() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_dword() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_qword() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_bool() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_u8() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_i8() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_u16() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_i16() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_u32() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_i32() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_u64() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_i64() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_usize() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_string() -> Result<()> {
        todo!()
    }

    #[test]
    fn test_write_raw() -> Result<()> {
        todo!()
    }

    // --------------------------------------------------------------
    // Serde types
    // --------------------------------------------------------------
    use serde::ser::Serializer;

    #[test]
    fn test_serializer_serialize_bool() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_bool(true)?;
        assert_eq!(buf, [1]);

        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_bool(false)?;
        assert_eq!(buf, [0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_i8() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_i8(42)?;
        assert_eq!(buf, [42]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_u8() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_u8(42)?;
        assert_eq!(buf, [42]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_i16() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_i16(42)?;
        assert_eq!(buf, [42, 0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_u16() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_u16(42)?;
        assert_eq!(buf, [42, 0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_i32() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_i32(42)?;
        assert_eq!(buf, [42, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_u32() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_u32(42)?;
        assert_eq!(buf, [42, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_i64() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_i64(42)?;
        assert_eq!(buf, [42, 0, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_u64() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_u64(42)?;
        assert_eq!(buf, [42, 0, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_f32() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_f32(1.0)?;
        assert_eq!(buf, [0xcd, 0xcc, 0xcc, 0x3d]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_f64() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_f64(1.0)?;
        assert_eq!(buf, [0x9a, 0x99, 0x99, 0x99, 0x99, 0x99, 0xb9, 0x3f]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_bytes() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_bytes(&[1, 2, 3, 4, 5])?;
        assert_eq!(buf, [5, 0, 0, 0, 1, 2, 3, 4, 5]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_option() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_some(&42i16)?;
        assert_eq!(buf, [1, 42, 0]);

        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_none()?;
        assert_eq!(buf, [0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_unit() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_unit()?;
        assert_eq!(buf, []);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_sequence() -> Result<()> {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&1216)?;
        seq.serialize_element(&22i16)?;
        seq.serialize_element(&23i16)?;
        seq.end()?;
        assert_eq!(buf, [3, 0, 0, 0, 12, 0, 22, 0, 23, 0]);
        Ok(())
    }

    #[test]
    fn test_serializer_serialize_sequence_unknown_size() {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        let mut seq = serializer.serialize_seq(None).unwrap();
        todo!()
    }

    #[test]
    fn test_serializer_serialize_sequence_bad_size() {
        let mut buf = Vec::new();
        let mut serializer = super::Serializer::from_writer(&mut buf);
        let mut seq = serializer.serialize_seq(None).unwrap();
        todo!()
    }

    #[test]
    fn test_serializer_serialize_tuple() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_map() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    // --------------------------------------------------------------
    // Equivalence types
    // --------------------------------------------------------------

    #[test]
    fn test_serializer_serialize_char() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_string() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_newtype_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_unit_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_tuple_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_unit_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_newtype_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_tuple_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }

    #[test]
    fn test_serializer_serialize_struct_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        todo!()
    }
}
