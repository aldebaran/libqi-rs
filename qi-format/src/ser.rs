/// Serialization of `serde` values in the `qi` format.
///
/// The following `serde` types are not handled:
///
/// - `i128`
/// - `u128`
use crate::{write::*, Error, Result};
use bytes::{Bytes, BytesMut};

pub fn to_bytes<T>(value: &T) -> Result<Bytes>
where
    T: serde::Serialize,
{
    let mut bytes = BytesMut::new();
    let serializer = Serializer::to_buf(&mut bytes);
    value.serialize(serializer)?;
    Ok(bytes.freeze())
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Serializer<'a, B> {
    bytes: &'a mut B,
}

impl<'a, B> Serializer<'a, B>
where
    B: bytes::BufMut,
{
    pub fn to_buf(bytes: &'a mut B) -> Self {
        Self { bytes }
    }
}

impl<'a, B> serde::Serializer for Serializer<'a, B>
where
    B: bytes::BufMut,
{
    type Ok = &'a mut B;
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a, B>;
    type SerializeTuple = SeqSerializer<'a, B>;
    type SerializeTupleStruct = SeqSerializer<'a, B>;
    type SerializeTupleVariant = SeqSerializer<'a, B>;
    type SerializeMap = SeqSerializer<'a, B>;
    type SerializeStruct = SeqSerializer<'a, B>;
    type SerializeStructVariant = SeqSerializer<'a, B>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        write_bool(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        write_i8(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        write_u8(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        write_i16(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        write_u16(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        write_i32(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        write_u32(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        write_i64(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        write_u64(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        write_f32(self.bytes, v);
        Ok(self.bytes)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        write_f64(self.bytes, v);
        Ok(self.bytes)
    }

    // bytes -> raw
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        write_raw(self.bytes, v)?;
        Ok(self.bytes)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        write_str(self.bytes, v)?;
        Ok(self.bytes)
    }

    // equivalence: char -> str
    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let buf = &mut [0; 4];
        let str = v.encode_utf8(buf);
        self.serialize_str(str)
    }

    // option -> optional
    fn serialize_none(self) -> Result<Self::Ok> {
        write_bool(self.bytes, false);
        Ok(self.bytes)
    }

    // option -> optional
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        write_bool(self.bytes, true);
        value.serialize(self)
    }

    // sequence -> list
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let size = len.ok_or(Error::UnspecifiedListMapSize)?;
        write_size(self.bytes, size)?;
        Ok(SeqSerializer::new(self.bytes, size))
    }

    // unit -> unit
    fn serialize_unit(self) -> Result<Self::Ok> {
        // nothing
        Ok(self.bytes)
    }

    // equivalence: unit_struct -> unit
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    // tuple -> tuple
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Ok(SeqSerializer::new(self.bytes, len))
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
        self.serialize_seq(len)
    }

    // equivalence: newtype_struct(T) -> tuple(T)
    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        let mut tuple_ser = self.serialize_tuple(1)?;
        use serde::ser::SerializeTuple;
        tuple_ser.serialize_element(value)?;
        tuple_ser.end()
    }

    // equivalence: tuple_variant(idx, T...) -> tuple(idx: uint_32, tuple(T...))
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        write_u32(self.bytes, variant_index);
        self.serialize_tuple(len)
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
        T: serde::Serialize,
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

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeqSerializer<'a, B> {
    buf: &'a mut B,
    size: usize,
    elements_left: usize,
}

impl<'a, B> SeqSerializer<'a, B>
where
    B: bytes::BufMut,
{
    fn new(buf: &'a mut B, size: usize) -> Self {
        Self {
            buf,
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
        value.serialize(Serializer::to_buf(self.buf))?;
        Ok(())
    }
}

impl<'a, B> serde::ser::SerializeSeq for SeqSerializer<'a, B>
where
    B: bytes::BufMut,
{
    type Ok = &'a mut B;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.buf)
    }
}

impl<'a, B> serde::ser::SerializeMap for SeqSerializer<'a, B>
where
    B: bytes::BufMut,
{
    type Ok = &'a mut B;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize(value)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.buf)
    }
}

impl<'a, B> serde::ser::SerializeTuple for SeqSerializer<'a, B>
where
    B: bytes::BufMut,
{
    type Ok = &'a mut B;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.buf)
    }
}

impl<'a, B> serde::ser::SerializeTupleStruct for SeqSerializer<'a, B>
where
    B: bytes::BufMut,
{
    type Ok = &'a mut B;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.buf)
    }
}

impl<'a, B> serde::ser::SerializeTupleVariant for SeqSerializer<'a, B>
where
    B: bytes::BufMut,
{
    type Ok = &'a mut B;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.buf)
    }
}

impl<'a, B> serde::ser::SerializeStruct for SeqSerializer<'a, B>
where
    B: bytes::BufMut,
{
    type Ok = &'a mut B;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.buf)
    }
}

impl<'a, B> serde::ser::SerializeStructVariant for SeqSerializer<'a, B>
where
    B: bytes::BufMut,
{
    type Ok = &'a mut B;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.try_decr_elements_left()?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.buf)
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
    use serde::ser::Serializer;

    // --------------------------------------------------------------
    // Bijection types
    // --------------------------------------------------------------

    #[test]
    fn test_serializer_serialize_bool() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_bool(true).unwrap();
        assert_eq!(buf, [1]);

        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_bool(false).unwrap();
        assert_eq!(buf, [0]);
    }

    #[test]
    fn test_serializer_serialize_i8() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_i8(42).unwrap();
        assert_eq!(buf, [42]);
    }

    #[test]
    fn test_serializer_serialize_u8() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_u8(42).unwrap();
        assert_eq!(buf, [42]);
    }

    #[test]
    fn test_serializer_serialize_i16() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_i16(42).unwrap();
        assert_eq!(buf, [42, 0]);
    }

    #[test]
    fn test_serializer_serialize_u16() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_u16(42).unwrap();
        assert_eq!(buf, [42, 0]);
    }

    #[test]
    fn test_serializer_serialize_i32() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_i32(42).unwrap();
        assert_eq!(buf, [42, 0, 0, 0]);
    }

    #[test]
    fn test_serializer_serialize_u32() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_u32(42).unwrap();
        assert_eq!(buf, [42, 0, 0, 0]);
    }

    #[test]
    fn test_serializer_serialize_i64() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_i64(42).unwrap();
        assert_eq!(buf, [42, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_serializer_serialize_u64() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_u64(42).unwrap();
        assert_eq!(buf, [42, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_serializer_serialize_f32() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_f32(1.0).unwrap();
        assert_eq!(buf, [0, 0, 128, 63]);
    }

    #[test]
    fn test_serializer_serialize_f64() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_f64(1.0).unwrap();
        assert_eq!(buf, [0, 0, 0, 0, 0, 0, 240, 63]);
    }

    #[test]
    fn test_serializer_serialize_bytes() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_bytes(&[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(buf, [5, 0, 0, 0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_serializer_serialize_option() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_some(&42i16).unwrap();
        assert_eq!(buf, [1, 42, 0]);

        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_none().unwrap();
        assert_eq!(buf, [0]);
    }

    #[test]
    fn test_serializer_serialize_unit() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_unit().unwrap();
        assert_eq!(buf, []);
    }

    #[test]
    fn test_serializer_serialize_sequence() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
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
        let serializer = super::Serializer::to_buf(&mut buf);
        assert_matches!(
            serializer.serialize_seq(None),
            Err(Error::UnspecifiedListMapSize)
        );
    }

    #[test]
    fn test_serializer_serialize_tuple() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
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
        let serializer = super::Serializer::to_buf(&mut buf);
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
        let serializer = super::Serializer::to_buf(&mut buf);
        assert_matches!(
            serializer.serialize_map(None),
            Err(Error::UnspecifiedListMapSize)
        );
    }

    // --------------------------------------------------------------
    // Equivalence types
    // --------------------------------------------------------------

    #[test]
    // char -> str
    fn test_serializer_serialize_char() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_char('a').unwrap();
        assert_eq!(buf, [1, 0, 0, 0, 97]);
    }

    #[test]
    // str -> bytes.
    fn test_serializer_serialize_str() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_str("abc").unwrap();
        assert_eq!(buf, [3, 0, 0, 0, 97, 98, 99]);
    }

    #[test]
    // struct -> tuple.
    fn test_serializer_serialize_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
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
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer
            .serialize_newtype_struct("MyStruct", &12i32)
            .unwrap();
        assert_eq!(buf, [12, 0, 0, 0]);
    }

    #[test]
    // unit_struct -> unit
    fn test_serializer_serialize_unit_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer.serialize_unit_struct("MyStruct").unwrap();
        assert_eq!(buf, []);
    }

    #[test]
    // tuple_struct -> tuple
    fn test_serializer_serialize_tuple_struct() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
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
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer
            .serialize_unit_variant("MyEnum", 23, "MyVariant")
            .unwrap();
        assert_eq!(buf, [23, 0, 0, 0])
    }

    #[test]
    // newtype_variant(T) -> tuple(uint32, T)
    fn test_serializer_serialize_newtype_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
        serializer
            .serialize_newtype_variant("MyEnum", 123, "MyVariant", "abc")
            .unwrap();
        assert_eq!(buf, [123, 0, 0, 0, 3, 0, 0, 0, 97, 98, 99]);
    }

    #[test]
    // tuple_variant(T...) -> tuple(uint32, tuple(T...)) = tuple(uint32, T...)
    fn test_serializer_serialize_tuple_variant() {
        let mut buf = Vec::new();
        let serializer = super::Serializer::to_buf(&mut buf);
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
        let serializer = super::Serializer::to_buf(&mut buf);
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
