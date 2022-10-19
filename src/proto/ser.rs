use super::{message::MagicCookie, Error, Message, Result};

pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: std::io::Write,
    T: ?Sized + serde::Serialize,
{
    value.serialize(&mut Serializer::from_writer(writer))
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + serde::Serialize,
{
    let mut buf = Vec::new();
    to_writer(&mut buf, value)?;
    Ok(buf)
}

pub fn to_message<T>(msg: &mut Message, value: &T) -> Result<()>
where
    T: ?Sized + serde::Serialize,
{
    to_writer(&mut msg.payload, value)
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

    fn write_u32(&mut self, v: u32) -> std::io::Result<()> {
        self.writer.write_all(&v.to_le_bytes())
    }

    fn write_size(&mut self, size: usize) -> Result<()> {
        // Sizes are always serialized as u32 in libqi.
        let size = size.try_into().map_err(|e| Error::BadSize(e))?;
        self.write_u32(size)?;
        Ok(())
    }

    fn serialize<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut Serializer {
            writer: self.writer.by_ref(),
        })
    }
}

impl<'ser, W> serde::Serializer for &'ser mut Serializer<W>
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
    type SerializeStruct = StructSerializer<'ser, W>;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.serialize_u8(if v { 1 } else { 0 })
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.serialize_u8(v as u8)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.writer.write_all(&[v])?;
        Ok(())
    }

    // LibQi does not handle endianness correctly, and as such always
    // serialize integers with native byte order. However, as it mostly
    // executes on little endian systems, we assume they are always
    // encoded as such, to ensure portability with systems that are not
    // little endian.

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
        self.write_u32(v)?;
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        self.writer.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.serialize_u32(v as u32)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.serialize_u64(v as u64)
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
        let mut seq = self.serialize_seq(Some(v.len()))?;
        use serde::ser::SerializeSeq;
        for b in v {
            seq.serialize_element(b)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.serialize_bool(false)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
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
        T: serde::Serialize,
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
        T: serde::Serialize,
    {
        self.write_u32(variant_index)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let size = len.ok_or(Error::UnknownListSize)?;
        self.write_size(size)?;
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.write_u32(variant_index)?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        self.serialize_seq(len)
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(match name {
            Message::TOKEN => MessageStructSerializer::from_writer(self.writer.by_ref()).into(),
            _ => self.into(),
        })
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
        T: serde::Serialize,
    {
        self.serialize(value)
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
        T: serde::Serialize,
    {
        self.serialize(value)
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
        T: serde::Serialize,
    {
        self.serialize(value)
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
        T: serde::Serialize,
    {
        self.serialize(value)
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
        T: serde::Serialize,
    {
        self.serialize(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize(value)
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
        T: serde::Serialize,
    {
        self.serialize(value)
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
        T: serde::Serialize,
    {
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

pub enum StructSerializer<'ser, W> {
    Message(MessageStructSerializer<&'ser mut W>),
    Other(&'ser mut Serializer<W>),
}

impl<'ser, W> serde::ser::SerializeStruct for StructSerializer<'ser, W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        match self {
            Self::Message(m) => m.serialize_field(key, value),
            Self::Other(s) => s.serialize_field(key, value),
        }
    }

    fn end(self) -> Result<Self::Ok> {
        match self {
            Self::Message(m) => m.end(),
            Self::Other(s) => s.end(),
        }
    }
}

impl<'ser, W> From<&'ser mut Serializer<W>> for StructSerializer<'ser, W> {
    fn from(s: &'ser mut Serializer<W>) -> Self {
        Self::Other(s)
    }
}

// # Message
// ## Structure
// ```text
// ╔═══════════════╤═══════════════╤═══════════════╤═══════════════╗
// ║       1       │       2       │       3       │       4       ║
// ╟─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─╢
// ║0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7║
// ╠═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╣
// ║                         magic cookie                          ║
// ╟───────────────────────────────────────────────────────────────╢
// ║                          identifier                           ║
// ╟───────────────────────────────────────────────────────────────╢
// ║                         payload size                          ║
// ╟───────────────────────────────┬───────────────┬───────────────╢
// ║            version            │     type      │    flags      ║
// ╟───────────────────────────────┴───────────────┴───────────────╢
// ║                            service                            ║
// ╟───────────────────────────────────────────────────────────────╢
// ║                            object                             ║
// ╟───────────────────────────────────────────────────────────────╢
// ║                            action                             ║
// ╟───────────────────────────────────────────────────────────────╢
// ║                            payload                            ║
// ║                             [...]                             ║
// ╚═══════════════════════════════════════════════════════════════╝
// ```
//
// ## Header fields
//  - magic cookie: uint32
//  - id: uint32
//  - size/len: uint32, size of the payload. may be 0
//  - version: uint16
//  - type: uint8
//  - flags: uint8
//  - service: uint32
//  - object: uint32
//  - action: uint32
pub struct MessageStructSerializer<W> {
    writer: W,
    id: Option<[u8; 4]>,
    payload_size: Option<[u8; 4]>,
    version: Option<[u8; 2]>,
    kind: Option<u8>, // aka type
    flags: Option<u8>,
    service: Option<[u8; 4]>,
    object: Option<[u8; 4]>,
    action: Option<[u8; 4]>,
    payload: Option<Vec<u8>>, // TODO: &[u8] ?
}

impl<W> MessageStructSerializer<W> {
    fn from_writer(writer: W) -> Self {
        Self {
            writer,
            id: None,
            payload_size: None,
            version: None,
            kind: None,
            flags: None,
            service: None,
            object: None,
            action: None,
            payload: None,
        }
    }

    fn set_id<T>(&mut self, id: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.id.is_some() {
            return Err(Error::DuplicateMessageField(Message::ID_TOKEN));
        }
        let mut buf = [0; 4];
        id.serialize(&mut Serializer::from_writer(buf.as_mut_slice()))?;
        self.id = Some(buf);
        Ok(())
    }

    fn set_version<T>(&mut self, ver: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.version.is_some() {
            return Err(Error::DuplicateMessageField(Message::VERSION_TOKEN));
        }
        let mut buf = [0; 2];
        ver.serialize(&mut Serializer::from_writer(buf.as_mut_slice()))?;
        self.version = Some(buf);
        Ok(())
    }

    fn set_kind<T>(&mut self, kind: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.kind.is_some() {
            return Err(Error::DuplicateMessageField(Message::KIND_TOKEN));
        }
        let mut buf = [0; 1];
        kind.serialize(&mut Serializer::from_writer(buf.as_mut_slice()))?;
        self.kind = Some(buf[0]);
        Ok(())
    }

    fn set_flags<T>(&mut self, flags: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.flags.is_some() {
            return Err(Error::DuplicateMessageField(Message::FLAGS_TOKEN));
        }
        let mut buf = [0; 1];
        flags.serialize(&mut Serializer::from_writer(buf.as_mut_slice()))?;
        self.flags = Some(buf[0]);
        Ok(())
    }

    fn set_subject<T>(&mut self, subject: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.service.is_some() || self.object.is_some() || self.action.is_some() {
            return Err(Error::DuplicateMessageField(Message::SUBJECT_TOKEN));
        }
        let mut buf = [0; 12];
        subject.serialize(&mut Serializer::from_writer(buf.as_mut_slice()))?;
        self.service = Some(buf[0..4].try_into().unwrap());
        self.object = Some(buf[4..8].try_into().unwrap());
        self.action = Some(buf[8..12].try_into().unwrap());
        Ok(())
    }

    fn set_payload<T>(&mut self, payload: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.payload_size.is_some() || self.payload.is_some() {
            return Err(Error::DuplicateMessageField(Message::PAYLOAD_TOKEN));
        }
        let mut buf = Vec::new();
        payload.serialize(&mut Serializer::from_writer(&mut buf))?;
        if buf.len() < 4 {
            return Err(Error::NoPayloadSize);
        }
        self.payload_size = Some(buf[0..4].try_into().unwrap());
        self.payload = Some(buf[4..].into());
        Ok(())
    }
}

impl<'ser, W> From<MessageStructSerializer<&'ser mut W>> for StructSerializer<'ser, W> {
    fn from(s: MessageStructSerializer<&'ser mut W>) -> Self {
        Self::Message(s)
    }
}

impl<W> serde::ser::SerializeStruct for MessageStructSerializer<W>
where
    W: std::io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + serde::Serialize,
    {
        match key {
            Message::ID_TOKEN => self.set_id(value),
            Message::VERSION_TOKEN => self.set_version(value),
            Message::KIND_TOKEN => self.set_kind(value),
            Message::FLAGS_TOKEN => self.set_flags(value),
            Message::SUBJECT_TOKEN => self.set_subject(value),
            Message::PAYLOAD_TOKEN => self.set_payload(value),
            _ => Err(Error::UnexpectedMessageField(key)),
        }
    }

    fn end(mut self) -> Result<Self::Ok> {
        let missing_field = |field| move || Error::MissingMessageField(field);
        let id = self.id.ok_or_else(missing_field(Message::ID_TOKEN))?;
        let version = self
            .version
            .ok_or_else(missing_field(Message::VERSION_TOKEN))?;
        let kind = self.kind.ok_or_else(missing_field(Message::KIND_TOKEN))?;
        let flags = self.flags.ok_or_else(missing_field(Message::FLAGS_TOKEN))?;
        let service = self
            .service
            .ok_or_else(missing_field(Message::SUBJECT_TOKEN))?;
        let object = self
            .object
            .ok_or_else(missing_field(Message::SUBJECT_TOKEN))?;
        let action = self
            .action
            .ok_or_else(missing_field(Message::SUBJECT_TOKEN))?;
        let payload_size = self
            .payload_size
            .ok_or_else(missing_field(Message::PAYLOAD_TOKEN))?;
        let payload = self
            .payload
            .ok_or_else(missing_field(Message::PAYLOAD_TOKEN))?;

        use serde::Serialize;
        MagicCookie.serialize(&mut Serializer::from_writer(self.writer.by_ref()))?;
        self.writer.write_all(&id)?;
        self.writer.write_all(&payload_size)?;
        self.writer.write_all(&version)?;
        self.writer.write_all(&[kind])?;
        self.writer.write_all(&[flags])?;
        self.writer.write_all(&service)?;
        self.writer.write_all(&object)?;
        self.writer.write_all(&action)?;
        self.writer.write_all(&payload)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_message_to_bytes() {
        use crate::proto::message::*;
        let msg = Message {
            id: 329,
            version: 12,
            kind: Kind::Capability,
            flags: Flags::RETURN_TYPE,
            subject: subject::ServiceDirectory {
                action: action::ServiceDirectory::ServiceReady,
            }
            .into(),
            payload: vec![0x17, 0x2b, 0xe6, 0x01, 0x5f],
        };
        let buf = to_bytes(&msg).unwrap();
        assert_eq!(
            buf,
            vec![
                0x42, 0xde, 0xad, 0x42, // cookie
                0x49, 0x01, 0x00, 0x00, // id
                0x05, 0x00, 0x00, 0x00, // size
                0x0c, 0x00, 0x06, 0x02, // version, type, flags
                0x01, 0x00, 0x00, 0x00, // service
                0x01, 0x00, 0x00, 0x00, // object
                0x68, 0x00, 0x00, 0x00, // action
                0x17, 0x2b, 0xe6, 0x01, 0x5f, // payload
            ]
        );
    }

    #[test]
    fn test_subject_to_bytes() {
        use crate::proto::message::subject::*;
        let subject =
            BoundObject::from_values_unchecked(service::Id(23), object::Id(923), action::Id(392));
        let buf = to_bytes(&subject).unwrap();
        assert_eq!(
            buf,
            vec![
                0x17, 0x00, 0x00, 0x00, // service
                0x9b, 0x03, 0x00, 0x00, // object
                0x88, 0x01, 0x00, 0x00, // action
            ]
        );
    }

    #[test]
    fn test_string_to_message() {
        let mut msg = Message::new();
        let data = "sample data";

        to_message(&mut msg, data).unwrap();
        assert_eq!(
            msg.payload,
            vec![
                0x0b, 0x00, 0x00, 0x00, // size
                0x73, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x20, 0x64, 0x61, 0x74, 0x61, 0x0a
            ]
        );
    }

    #[test]
    fn test_option_i32_to_bytes() {
        assert_eq!(
            to_bytes(&Some(42)).unwrap(),
            vec![0x01, 0x2a, 0x00, 0x00, 0x00]
        );
        assert_eq!(to_bytes(&Option::<i32>::None).unwrap(), vec![0x00]);
    }
}
