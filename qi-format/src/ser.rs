use crate::{
    num_bool::{FALSE_BOOL, TRUE_BOOL},
    Error, Result,
};
use serde::Serialize;

pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: std::io::Write,
    T: ?Sized + Serialize,
{
    let serializer = Serializer::from_writer(writer);
    value.serialize(serializer)?;
    Ok(())
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut buf = Vec::new();
    to_writer(&mut buf, value)?;
    Ok(buf)
}

pub fn write_byte<W>(mut writer: W, b: u8) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(&[b])?;
    Ok(())
}

pub fn write_word<W>(mut writer: W, w: &[u8; 2]) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(w)?;
    Ok(())
}

pub fn write_dword<W>(mut writer: W, dw: &[u8; 4]) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(dw)?;
    Ok(())
}

pub fn write_qword<W>(mut writer: W, qw: &[u8; 8]) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(qw)?;
    Ok(())
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
    write_byte(writer, val)
}

pub fn write_i8<W>(writer: W, val: i8) -> Result<()>
where
    W: std::io::Write,
{
    write_byte(writer, val as u8)
}

pub fn write_u16<W>(writer: W, val: u16) -> Result<()>
where
    W: std::io::Write,
{
    write_word(writer, &val.to_le_bytes())
}

pub fn write_i16<W>(writer: W, val: i16) -> Result<()>
where
    W: std::io::Write,
{
    write_word(writer, &val.to_le_bytes())
}

pub fn write_u32<W>(writer: W, val: u32) -> Result<()>
where
    W: std::io::Write,
{
    write_dword(writer, &val.to_le_bytes())
}

pub fn write_i32<W>(writer: W, val: i32) -> Result<()>
where
    W: std::io::Write,
{
    write_dword(writer, &val.to_le_bytes())
}

pub fn write_u64<W>(writer: W, val: u64) -> Result<()>
where
    W: std::io::Write,
{
    write_qword(writer, &val.to_le_bytes())
}

pub fn write_i64<W>(writer: W, val: i64) -> Result<()>
where
    W: std::io::Write,
{
    write_qword(writer, &val.to_le_bytes())
}

pub fn write_f32<W>(writer: W, val: f32) -> Result<()>
where
    W: std::io::Write,
{
    write_dword(writer, &val.to_le_bytes())
}

pub fn write_f64<W>(writer: W, val: f64) -> Result<()>
where
    W: std::io::Write,
{
    write_qword(writer, &val.to_le_bytes())
}

pub fn write_size<W>(writer: W, size: usize) -> Result<()>
where
    W: std::io::Write,
{
    let size = std::convert::TryFrom::try_from(size).map_err(Error::BadSize)?;
    write_u32(writer, size)
}

// equivalence: string -> raw
pub fn write_string<W>(writer: W, str: &str) -> Result<()>
where
    W: std::io::Write,
{
    write_raw(writer, str.as_bytes())
}

pub fn write_raw<W>(mut writer: W, raw: &[u8]) -> Result<()>
where
    W: std::io::Write,
{
    write_size(writer.by_ref(), raw.len())?;
    writer.write_all(raw)?;
    Ok(())
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
}

impl<W> serde::Serializer for Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<W>;
    type SerializeTuple = SeqSerializer<W>;
    type SerializeTupleStruct = SeqSerializer<W>;
    type SerializeTupleVariant = SeqSerializer<W>;
    type SerializeMap = SeqSerializer<W>;
    type SerializeStruct = SeqSerializer<W>;
    type SerializeStructVariant = SeqSerializer<W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        write_bool(self.writer, v)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        write_i8(self.writer, v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        write_u8(self.writer, v)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        write_i16(self.writer, v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        write_u16(self.writer, v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        write_i32(self.writer, v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        write_u32(self.writer, v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        write_i64(self.writer, v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        write_u64(self.writer, v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        write_f32(self.writer, v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        write_f64(self.writer, v)
    }

    // bytes -> raw
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        write_raw(self.writer, v)
    }

    // simple type: str
    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        write_string(self.writer, v)
    }

    // equivalence: char -> string = raw
    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let buf = &mut [0; 4];
        let str = v.encode_utf8(buf);
        write_string(self.writer, str)
    }

    // option -> optional
    fn serialize_none(self) -> Result<Self::Ok> {
        write_bool(self.writer, false)
    }

    // option -> optional
    fn serialize_some<T: ?Sized>(mut self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        write_bool(self.writer.by_ref(), true)?;
        value.serialize(self)
    }

    // sequence -> list
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let size = len.ok_or(Error::SizeIsRequired)?;
        let list_ser = SeqSerializer::new_list_or_map(self.writer, size)?;
        Ok(list_ser)
    }

    // unit -> unit
    fn serialize_unit(self) -> Result<Self::Ok> {
        // nothing
        Ok(())
    }

    // equivalence: unit_struct -> unit
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    // tuple -> tuple
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Ok(SeqSerializer::new_tuple(self.writer, len))
    }

    // equivalence: tuple_struct(T...) -> tuple(T...)
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    // equivalence: struct(T...) -> tuple(T...)
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_tuple(len)
    }

    // map(T,U) -> map(T,U)
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        let size = len.ok_or(Error::SizeIsRequired)?;
        let map_ser = SeqSerializer::new_list_or_map(self.writer, size)?;
        Ok(map_ser)
    }

    // equivalence: struct(T...) -> tuple(T...)
    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    // equivalence: tuple_variant(idx, T...) -> tuple(idx: uint_32, tuple(T...)) = tuple(idx, T...)
    fn serialize_tuple_variant(
        mut self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        write_u32(self.writer.by_ref(), variant_index)?;
        Ok(SeqSerializer::new_tuple(self.writer, len))
    }

    // equivalence: unit_variant(idx) -> tuple(idx: uint_32, unit) = tuple_variant(idx, unit)
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        let mut tuple = self.serialize_tuple_variant(name, variant_index, variant, 1)?;
        use serde::ser::SerializeTupleVariant;
        tuple.serialize_field(&())?;
        tuple.end()
    }

    // equivalence: newtype_variant(idx, T) -> tuple(idx: uint_32, tuple(T)) = tuple_variant(idx, T)
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        let mut tuple = self.serialize_tuple_variant(name, variant_index, variant, 1)?;
        use serde::ser::SerializeTupleVariant;
        tuple.serialize_field(value)?;
        tuple.end()
    }

    // equivalence: struct_variant(idx, T...) -> tuple(idx: uint_32, tuple(T...)) = tuple_variant(idx, T...)
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

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct SeqSerializer<W> {
    writer: W,
    size: usize,
    elements_left: usize,
}

impl<W> SeqSerializer<W>
where
    W: std::io::Write,
{
    fn new_list_or_map(mut writer: W, size: usize) -> Result<Self> {
        write_size(writer.by_ref(), size)?;
        Ok(Self {
            writer,
            size,
            elements_left: size,
        })
    }

    fn new_tuple(writer: W, size: usize) -> Self {
        Self {
            writer,
            size,
            elements_left: size,
        }
    }

    fn try_decr_elements_left(&mut self) -> Result<()> {
        match &mut self.elements_left {
            0 => return Err(Error::UnexpectedElement(self.size)),
            elements_left => *elements_left -= 1,
        }
        Ok(())
    }

    fn serialize<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(Serializer::from_writer(self.writer.by_ref()))
    }
}

impl<W> serde::ser::SerializeSeq for SeqSerializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeMap for SeqSerializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeTuple for SeqSerializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeTupleStruct for SeqSerializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeTupleVariant for SeqSerializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeStruct for SeqSerializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W> serde::ser::SerializeStructVariant for SeqSerializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
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
    use assert_matches::assert_matches;

    // --------------------------------------------------------------
    // Qi simple types
    // --------------------------------------------------------------

    #[test]
    fn test_write_byte() {
        let mut buf = Vec::new();
        write_byte(&mut buf, 64).unwrap();
        assert_eq!(buf, [64]);
        write_byte(&mut buf, 65).unwrap();
        assert_eq!(buf, [64, 65]);
    }

    #[test]
    fn test_write_word() {
        let mut buf = Vec::new();
        write_word(&mut buf, &[1, 2]).unwrap();
        assert_eq!(buf, [1, 2]);
        write_word(&mut buf, &[3, 4]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4]);
    }

    #[test]
    fn test_write_dword() {
        let mut buf = Vec::new();
        write_dword(&mut buf, &[1, 2, 3, 4]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4]);
        write_dword(&mut buf, &[5, 6, 7, 8]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_write_qword() {
        let mut buf = Vec::new();
        write_qword(&mut buf, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4, 5, 6, 7, 8]);
        write_qword(&mut buf, &[9, 10, 11, 12, 13, 14, 15, 16]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    }

    #[test]
    fn test_write_bool() {
        let mut buf = Vec::new();
        write_bool(&mut buf, true).unwrap();
        assert_eq!(buf, [1]);
        write_bool(&mut buf, false).unwrap();
        assert_eq!(buf, [1, 0]);
    }

    #[test]
    fn test_write_u8() {
        let mut buf = Vec::new();
        write_u8(&mut buf, 2).unwrap();
        assert_eq!(buf, [2]);
    }

    #[test]
    fn test_write_i8() {
        let mut buf = Vec::new();
        write_i8(&mut buf, -2).unwrap();
        assert_eq!(buf, [254]);
    }

    #[test]
    fn test_write_u16() {
        let mut buf = Vec::new();
        write_u16(&mut buf, 2).unwrap();
        assert_eq!(buf, [2, 0]);
    }

    #[test]
    fn test_write_i16() {
        let mut buf = Vec::new();
        write_i16(&mut buf, -2).unwrap();
        assert_eq!(buf, [254, 255]);
    }

    #[test]
    fn test_write_u32() {
        let mut buf = Vec::new();
        write_u32(&mut buf, 2).unwrap();
        assert_eq!(buf, [2, 0, 0, 0]);
    }

    #[test]
    fn test_write_i32() {
        let mut buf = Vec::new();
        write_i32(&mut buf, -2).unwrap();
        assert_eq!(buf, [254, 255, 255, 255]);
    }

    #[test]
    fn test_write_u64() {
        let mut buf = Vec::new();
        write_u64(&mut buf, 2).unwrap();
        assert_eq!(buf, [2, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_write_i64() {
        let mut buf = Vec::new();
        write_i64(&mut buf, -2).unwrap();
        assert_eq!(buf, [254, 255, 255, 255, 255, 255, 255, 255]);
    }

    #[test]
    fn test_write_f32() {
        let mut buf = Vec::new();
        write_f32(&mut buf, 1.0).unwrap();
        assert_eq!(buf, [0, 0, 128, 63]);
    }

    #[test]
    fn test_write_f64() {
        let mut buf = Vec::new();
        write_f64(&mut buf, 1.0).unwrap();
        assert_eq!(buf, [0, 0, 0, 0, 0, 0, 240, 63]);
    }

    #[test]
    fn test_write_size() {
        let mut buf = Vec::new();
        write_size(&mut buf, 2).unwrap();
        assert_eq!(buf, [2, 0, 0, 0]);
    }

    #[test]
    fn test_write_string() {
        let mut buf = Vec::new();
        write_string(&mut buf, "abc").unwrap();
        assert_eq!(buf, [3, 0, 0, 0, 97, 98, 99]);
    }

    #[test]
    fn test_write_raw() {
        let mut buf = Vec::new();
        write_raw(&mut buf, &[1, 11, 111]).unwrap();
        assert_eq!(buf, [3, 0, 0, 0, 1, 11, 111]);
    }

    // --------------------------------------------------------------
    // Serde types
    // --------------------------------------------------------------
    use serde::ser::Serializer;

    #[test]
    fn test_serializer_serialize_bool() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_bool(true).unwrap();
        assert_eq!(buf, [1]);

        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_bool(false).unwrap();
        assert_eq!(buf, [0]);
    }

    #[test]
    fn test_serializer_serialize_i8() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_i8(42).unwrap();
        assert_eq!(buf, [42]);
    }

    #[test]
    fn test_serializer_serialize_u8() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_u8(42).unwrap();
        assert_eq!(buf, [42]);
    }

    #[test]
    fn test_serializer_serialize_i16() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_i16(42).unwrap();
        assert_eq!(buf, [42, 0]);
    }

    #[test]
    fn test_serializer_serialize_u16() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_u16(42).unwrap();
        assert_eq!(buf, [42, 0]);
    }

    #[test]
    fn test_serializer_serialize_i32() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_i32(42).unwrap();
        assert_eq!(buf, [42, 0, 0, 0]);
    }

    #[test]
    fn test_serializer_serialize_u32() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_u32(42).unwrap();
        assert_eq!(buf, [42, 0, 0, 0]);
    }

    #[test]
    fn test_serializer_serialize_i64() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_i64(42).unwrap();
        assert_eq!(buf, [42, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_serializer_serialize_u64() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_u64(42).unwrap();
        assert_eq!(buf, [42, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_serializer_serialize_f32() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_f32(1.0).unwrap();
        assert_eq!(buf, [0, 0, 128, 63]);
    }

    #[test]
    fn test_serializer_serialize_f64() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_f64(1.0).unwrap();
        assert_eq!(buf, [0, 0, 0, 0, 0, 0, 240, 63]);
    }

    #[test]
    fn test_serializer_serialize_bytes() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_bytes(&[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(buf, [5, 0, 0, 0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_serializer_serialize_option() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_some(&42i16).unwrap();
        assert_eq!(buf, [1, 42, 0]);

        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_none().unwrap();
        assert_eq!(buf, [0]);
    }

    #[test]
    fn test_serializer_serialize_unit() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_unit().unwrap();
        assert_eq!(buf, []);
    }

    #[test]
    fn test_serializer_serialize_sequence() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(3)).unwrap();
        seq.serialize_element(&12i16).unwrap();
        seq.serialize_element(&22i16).unwrap();
        seq.serialize_element(&23i16).unwrap();
        // More elements result in error.
        assert_matches!(seq.serialize_element(&3), Err(Error::UnexpectedElement(3)));
        seq.end().unwrap();
        assert_eq!(buf, [3, 0, 0, 0, 12, 0, 22, 0, 23, 0]);
    }

    #[test]
    fn test_serializer_serialize_sequence_unknown_size() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        assert_matches!(serializer.serialize_seq(None), Err(Error::SizeIsRequired));
    }

    #[test]
    fn test_serializer_serialize_tuple() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        use serde::ser::SerializeTuple;
        let mut tuple = serializer.serialize_tuple(2).unwrap();
        tuple.serialize_element(&42i16).unwrap();
        tuple.serialize_element(&true).unwrap();
        // More elements result in error.
        assert_matches!(
            tuple.serialize_element(&1290u32),
            Err(Error::UnexpectedElement(2))
        );
        tuple.end().unwrap();
        assert_eq!(buf, [42, 0, 1]);
    }

    #[test]
    fn test_serializer_serialize_map() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2)).unwrap();
        map.serialize_entry(&31, &false).unwrap();
        map.serialize_entry(&64, &true).unwrap();
        // More elements result in error.
        assert_matches!(
            map.serialize_entry(&123u8, &246u16),
            Err(Error::UnexpectedElement(2))
        );
        map.end().unwrap();
        assert_eq!(buf, [2, 0, 0, 0, 31, 0, 0, 0, 0, 64, 0, 0, 0, 1]);
    }

    #[test]
    fn test_serializer_serialize_map_unknown_size() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        assert_matches!(serializer.serialize_map(None), Err(Error::SizeIsRequired));
    }

    // --------------------------------------------------------------
    // Equivalence types
    // --------------------------------------------------------------

    #[test]
    // char -> bytes.
    fn test_serializer_serialize_char() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_char('a').unwrap();
        assert_eq!(buf, [1, 0, 0, 0, 97]);
    }

    #[test]
    // str -> bytes.
    fn test_serializer_serialize_str() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_str("abc").unwrap();
        assert_eq!(buf, [3, 0, 0, 0, 97, 98, 99]);
    }

    #[test]
    // struct -> tuple.
    fn test_serializer_serialize_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        use serde::ser::SerializeStruct;
        let mut st = serializer.serialize_struct("MyStruct", 2).unwrap();
        st.serialize_field("w", &23u16).unwrap();
        st.serialize_field("c", &'a').unwrap();
        // More fields result in errors.
        assert_matches!(
            st.serialize_field("c", &'a'),
            Err(Error::UnexpectedElement(2))
        );
        st.end().unwrap();
        assert_eq!(buf, [23, 0, 1, 0, 0, 0, 97]);
    }

    #[test]
    // newtype_struct -> tuple(T) = T
    fn test_serializer_serialize_newtype_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer
            .serialize_newtype_struct("MyStruct", &12i32)
            .unwrap();
        assert_eq!(buf, [12, 0, 0, 0]);
    }

    #[test]
    // unit_struct -> unit
    fn test_serializer_serialize_unit_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer.serialize_unit_struct("MyStruct").unwrap();
        assert_eq!(buf, []);
    }

    #[test]
    // tuple_struct -> tuple
    fn test_serializer_serialize_tuple_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        use serde::ser::SerializeTupleStruct;
        let mut tuple = serializer.serialize_tuple_struct("MyStruct", 2).unwrap();
        tuple.serialize_field(&234u16).unwrap();
        tuple.serialize_field(&'a').unwrap();
        // More elements result in error.
        assert_matches!(
            tuple.serialize_field(&true),
            Err(Error::UnexpectedElement(2))
        );
        tuple.end().unwrap();
        assert_eq!(buf, [234, 0, 1, 0, 0, 0, 97]);
    }

    #[test]
    // unit_variant -> tuple(uint32) = uint32
    fn test_serializer_serialize_unit_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer
            .serialize_unit_variant("MyEnum", 23, "MyVariant")
            .unwrap();
        assert_eq!(buf, [23, 0, 0, 0])
    }

    #[test]
    // newtype_variant(T) -> tuple(uint32, T)
    fn test_serializer_serialize_newtype_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        serializer
            .serialize_newtype_variant("MyEnum", 123, "MyVariant", "abc")
            .unwrap();
        assert_eq!(buf, [123, 0, 0, 0, 3, 0, 0, 0, 97, 98, 99]);
    }

    #[test]
    // tuple_variant(T...) -> tuple(uint32, tuple(T...)) = tuple(uint32, T...)
    fn test_serializer_serialize_tuple_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        use serde::ser::SerializeTupleVariant;
        let mut tuple = serializer
            .serialize_tuple_variant("MyEnum", 913, "MyVariant", 2)
            .unwrap();
        tuple.serialize_field(&3290u16).unwrap();
        tuple.serialize_field("def").unwrap();
        // More elements result in error.
        assert_matches!(
            tuple.serialize_field(&true),
            Err(Error::UnexpectedElement(2))
        );
        assert_eq!(buf, [145, 3, 0, 0, 218, 12, 3, 0, 0, 0, 100, 101, 102]);
    }

    #[test]
    // struct_variant(T...) -> tuple(uint32, tuple(T...)) = tuple(uint32, T...)
    fn test_serializer_serialize_struct_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::from_writer(&mut buf);
        use serde::ser::SerializeStructVariant;
        let mut st = serializer
            .serialize_struct_variant("MyEnum", 128, "MyVariant", 3)
            .unwrap();
        st.serialize_field("t", &(1u8, 2u8)).unwrap();
        st.serialize_field("c", &'1').unwrap();
        st.serialize_field("b", &true).unwrap();
        // More elements result in error.
        assert_matches!(
            st.serialize_field("s", "abc"),
            Err(Error::UnexpectedElement(3))
        );
        assert_eq!(buf, [128, 0, 0, 0, 1, 2, 1, 0, 0, 0, 49, 1]);
    }
}
