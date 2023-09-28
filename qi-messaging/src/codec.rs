//! Message encoding
//!
//! ## Message Structure
//!
//! ```text
//! ╔═══════════════════════════════════════════════════════════════════╗
//! ║                              HEADER                               ║
//! ╠═╤═══════════════╤═══════════════╤═══════════════╤═══════════════╤═╣
//! ║ │       0       │       1       │       2       │       3       │ ║
//! ║ ├─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┤ ║
//! ║ │0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│ ║
//! ║ ├─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┤ ║
//! ║ │                         magic cookie                          │ ║
//! ║ ├───────────────────────────────────────────────────────────────┤ ║
//! ║ │                          identifier                           │ ║
//! ║ ├───────────────────────────────────────────────────────────────┤ ║
//! ║ │                           body size                           │ ║
//! ║ ├───────────────────────────────┬───────────────┬───────────────┤ ║
//! ║ │            version            │     type      │    flags      │ ║
//! ║ ├───────────────────────────────┴───────────────┴───────────────┤ ║
//! ║ │                            service                            │ ║
//! ║ ├ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - ┤ ║
//! ║ │                            object                             │ ║
//! ║ ├ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - ┤ ║
//! ║ │                            action                             │ ║
//! ╠═╧═══════════════════════════════════════════════════════════════╧═╣
//! ║                               BODY                                ║
//! ╚═══════════════════════════════════════════════════════════════════╝
//! ```
//!
//! ### Header fields
//!  - magic cookie: 4 bytes, 0x42dead42 as big endian
//!  - id: 4 bytes unsigned integer, little endian
//!  - size/len: 4 bytes unsigned integer, size of the body. may be 0, little endian
//!  - version: 2 bytes unsigned integer, little endian
//!  - type: 1 byte unsigned integer
//!  - flags: 1 byte unsigned integer
//!  - address, 3 x 4 bytes unsigned integer, all little endian
//!    - service
//!    - object
//!    - action
//!
//!  The total header size is therefore 28 bytes.

use crate::{
    format,
    message::{Action, Address, Flags, Header, Id, Message, Object, Service, Type, Version},
};
use bytes::{Buf, BufMut, BytesMut};
use tracing::instrument;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub struct Codec {
    state: DecoderState,
}

impl Codec {
    pub(crate) fn new() -> Self {
        Self {
            state: DecoderState::Header,
        }
    }
}

impl Default for Codec {
    fn default() -> Self {
        Self::new()
    }
}

impl tokio_util::codec::Encoder<Message> for Codec {
    type Error = EncodeError;

    #[instrument(level = "trace", name = "encode", skip_all, err)]
    fn encode(&mut self, msg: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        put_message(msg, dst)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error(
        "message body size {0} cannot be represented as an u32 (the maximum for this system is {})",
        u32::MAX
    )]
    BodySizeCannotBeRepresentedAsU32Error(usize),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

impl tokio_util::codec::Decoder for Codec {
    type Item = Message;
    type Error = DecodeError;

    #[instrument(level = "trace", name = "decode", skip_all, err)]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let msg = loop {
            match self.state {
                DecoderState::Header => match decode_header(src)? {
                    None => break None,
                    Some(header) => self.state = DecoderState::Body(header),
                },
                DecoderState::Body(header) => match decode_body(header.body_size, src) {
                    None => break None,
                    Some(body) => {
                        self.state = DecoderState::Header;
                        src.reserve(src.len());
                        break Some(Message::new(header, body));
                    }
                },
            }
        };
        Ok(msg)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("invalid message magic cookie value {0:x}")]
    InvalidMagicCookieValue(u32),

    #[error(
        "message body size {0} cannot be represented as an usize (the maximum for this system is {})",
        usize::MAX
    )]
    BodySizeCannotBeRepresentedAsUSize(u32),

    #[error("unsupported message version {0}")]
    UnsupportedVersion(Version),

    #[error("invalid message type value {0}")]
    InvalidTypeValue(u8),

    #[error("invalid message flags value {0}")]
    InvalidFlagsValue(u8),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
enum DecoderState {
    Header,
    Body(Header),
}

fn put_message(msg: Message, dst: &mut BytesMut) -> Result<(), EncodeError> {
    let msg_size = HEADER_SIZE + msg.body_size();
    dst.reserve(msg_size);
    put_header(msg.header(), dst)?;
    put_body(msg.body(), dst);
    Ok(())
}

fn decode_header(src: &mut BytesMut) -> Result<Option<Header>, DecodeError> {
    if src.len() < HEADER_SIZE {
        src.reserve(HEADER_SIZE - src.len());
        return Ok(None);
    }

    get_magic_cookie(src)?;
    let id = get_id(src);
    let body_size = get_body_size(src)?;
    let version = get_version(src);

    // The only supported version of messages at the moment is the current.
    if version != Version::current() {
        return Err(DecodeError::UnsupportedVersion(version));
    }

    let ty = get_type(src)?;
    let flags = get_flags(src)?;
    let address = get_address(src);
    let header = Header {
        id,
        ty,
        body_size,
        version,
        flags,
        address,
    };
    Ok(Some(header))
}

fn put_header(header: Header, dst: &mut BytesMut) -> Result<(), EncodeError> {
    put_magic_cookie(dst);
    put_id(header.id, dst);
    put_body_size(header.body_size, dst)?;
    put_version(Version::current(), dst);
    put_type(header.ty, dst);
    put_flags(header.flags, dst);
    put_address(header.address, dst);
    Ok(())
}

// Sizes of header fields in bytes.
const MAGIC_COOKIE_SIZE: usize = 4;
const ID_SIZE: usize = 4;
const BODY_SIZE_SIZE: usize = 4;
const VERSION_SIZE: usize = 2;
const TYPE_SIZE: usize = 1;
const FLAGS_SIZE: usize = 1;
const ADDRESS_SIZE: usize = 12;

// Offsets of header fields in byte distances.
const MAGIC_COOKIE_OFFSET: usize = 0;
const ID_OFFSET: usize = MAGIC_COOKIE_OFFSET + MAGIC_COOKIE_SIZE;
const BODY_SIZE_OFFSET: usize = ID_OFFSET + ID_SIZE;
const VERSION_OFFSET: usize = BODY_SIZE_OFFSET + BODY_SIZE_SIZE;
const TYPE_OFFSET: usize = VERSION_OFFSET + VERSION_SIZE;
const FLAGS_OFFSET: usize = TYPE_OFFSET + TYPE_SIZE;
const ADDRESS_OFFSET: usize = FLAGS_OFFSET + FLAGS_SIZE;
const HEADER_SIZE: usize = ADDRESS_OFFSET + ADDRESS_SIZE;

const MAGIC_COOKIE_VALUE: u32 = 0x42dead42;

fn get_magic_cookie(src: &mut BytesMut) -> Result<(), DecodeError> {
    let value = src.get_u32();
    if value == MAGIC_COOKIE_VALUE {
        Ok(())
    } else {
        Err(DecodeError::InvalidMagicCookieValue(value))
    }
}

fn put_magic_cookie(dst: &mut BytesMut) {
    dst.put_u32(MAGIC_COOKIE_VALUE)
}

fn get_id(src: &mut BytesMut) -> Id {
    Id(src.get_u32_le())
}

fn put_id(id: Id, dst: &mut BytesMut) {
    dst.put_u32_le(id.0)
}

fn get_body_size(src: &mut BytesMut) -> Result<usize, DecodeError> {
    let size = src.get_u32_le();
    if size > (usize::MAX as u32) {
        return Err(DecodeError::BodySizeCannotBeRepresentedAsUSize(size));
    }
    let size = size as usize;
    Ok(size)
}

fn put_body_size(size: usize, dst: &mut BytesMut) -> Result<(), EncodeError> {
    if size > (u32::MAX as usize) {
        return Err(EncodeError::BodySizeCannotBeRepresentedAsU32Error(size));
    }
    let size = size as u32;
    dst.put_u32_le(size);
    Ok(())
}

fn get_version(src: &mut BytesMut) -> Version {
    Version(src.get_u16_le())
}

fn put_version(version: Version, dst: &mut BytesMut) {
    dst.put_u16_le(version.0)
}

// Type representation values:
//   - Call = 1
//   - Reply = 2
//   - Error = 3
//   - Post = 4
//   - Event = 5
//   - Capabilities = 6
//   - Cancel = 7
//   - Canceled = 8

fn get_type(src: &mut BytesMut) -> Result<Type, DecodeError> {
    let byte = src.get_u8();
    match byte {
        1 => Ok(Type::Call),
        2 => Ok(Type::Reply),
        3 => Ok(Type::Error),
        4 => Ok(Type::Post),
        5 => Ok(Type::Event),
        6 => Ok(Type::Capabilities),
        7 => Ok(Type::Cancel),
        8 => Ok(Type::Canceled),
        _ => Err(DecodeError::InvalidTypeValue(byte)),
    }
}

fn put_type(ty: Type, dst: &mut BytesMut) {
    let ty_u8: u8 = match ty {
        Type::Call => 1,
        Type::Reply => 2,
        Type::Error => 3,
        Type::Post => 4,
        Type::Event => 5,
        Type::Capabilities => 6,
        Type::Cancel => 7,
        Type::Canceled => 8,
    };
    dst.put_u8(ty_u8)
}

fn get_flags(src: &mut BytesMut) -> Result<Flags, DecodeError> {
    let byte = src.get_u8();
    let flags = Flags::from_bits(byte).ok_or(DecodeError::InvalidFlagsValue(byte))?;
    Ok(flags)
}

fn put_flags(flags: Flags, dst: &mut BytesMut) {
    dst.put_u8(flags.bits())
}

fn get_address(src: &mut BytesMut) -> Address {
    let service = Service::from(src.get_u32_le());
    let object = Object::from(src.get_u32_le());
    let action = Action::from(src.get_u32_le());
    Address {
        service,
        object,
        action,
    }
}

fn put_address(address: Address, dst: &mut BytesMut) {
    dst.put_u32_le(address.service.into());
    dst.put_u32_le(address.object.into());
    dst.put_u32_le(address.action.into());
}

fn decode_body(size: usize, src: &mut BytesMut) -> Option<format::Value> {
    if src.len() < size {
        src.reserve(size - src.len());
        return None;
    }
    let bytes = src.copy_to_bytes(size);
    let value = format::Value::from_bytes(bytes);
    Some(value)
}

fn put_body(body: format::Value, dst: &mut BytesMut) {
    dst.put_slice(body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message;
    use assert_matches::assert_matches;

    #[test]
    fn test_encoder_success() {
        let message = Message {
            id: message::Id(1),
            ty: message::Type::Call,
            version: Version::current(),
            address: message::Address::default(),
            flags: message::Flags::all(),
            body: [1, 2, 3].into(),
        };
        let mut encoder_buf = BytesMut::new();
        let mut encoder = Codec::new();
        let res =
            tokio_util::codec::Encoder::encode(&mut encoder, message.clone(), &mut encoder_buf);
        assert_matches!(res, Ok(()));
    }

    #[test]
    fn test_decoder_not_enough_data_for_header() {
        let data = [0x42, 0xde, 0xad];
        let mut buf = BytesMut::from_iter(data);
        let mut decoder = Codec::new();
        let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
        assert_matches!(res, Ok(None));
    }

    #[test]
    fn test_decoder_not_enough_data_for_body() {
        let data = [
            0x42, 0xde, 0xad, 0x42, // cookie
            1, 0, 0, 0, // id
            5, 0, 0, 0, // size
            0, 0, 6, 2, // version, type, flags
            1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, // address,
            1, 2, 3, // body
        ];
        let mut buf = BytesMut::from_iter(data);
        let mut decoder = Codec::new();
        let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
        assert_matches!(res, Ok(None));
    }

    #[test]
    fn test_decoder_garbage_magic_cookie() {
        let data = [1; HEADER_SIZE];
        let mut buf = BytesMut::from_iter(data);
        let mut decoder = Codec::new();
        let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
        assert_matches!(res, Err(DecodeError::InvalidMagicCookieValue(0x01010101)));
    }

    #[test]
    fn test_decoder_success() {
        let data = [
            0x42, 0xde, 0xad, 0x42, // cookie
            1, 0, 0, 0, // id
            4, 0, 0, 0, // size
            0, 0, 6, 2, // version, type, flags
            1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, // address,
            1, 2, 3, 4, // body
        ];
        let mut buf = BytesMut::from_iter(data);
        let mut decoder = Codec::new();
        let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
        assert_matches!(res, Ok(Some(_msg)));
    }

    #[test]
    fn test_header_size() {
        assert_eq!(HEADER_SIZE, 28);
    }

    #[test]
    fn test_header_decode() {
        let mut input = [
            0x42, 0xde, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
        ]
        .as_slice()
        .into();
        let header = decode_header(&mut input).unwrap().unwrap();
        assert_eq!(
            header,
            Header {
                id: Id(990340),
                ty: Type::Error,
                body_size: 35,
                version: Version::current(),
                address: Address {
                    service: Service(47),
                    object: Object(1),
                    action: Action(178)
                },
                flags: Flags::empty(),
            }
        );
    }

    #[test]
    fn test_message_encode() {
        let msg = Message {
            id: Id(329),
            ty: Type::Capabilities,
            version: Version::current(),
            address: Address {
                service: Service(1),
                object: Object(1),
                action: Action(104),
            },
            flags: Flags::RETURN_TYPE,
            body: [0x17, 0x2b, 0xe6, 0x01, 0x5f].into(),
        };
        let mut buf = BytesMut::new();
        put_message(msg, &mut buf).unwrap();

        assert_eq!(
            buf,
            [
                0x42, 0xde, 0xad, 0x42, // cookie
                0x49, 0x01, 0x00, 0x00, // id
                0x05, 0x00, 0x00, 0x00, // size
                0x00, 0x00, 0x06, 0x02, // version, type, flags
                0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, 0x00, 0x00,
                0x00, // address,
                0x17, 0x2b, 0xe6, 0x01, 0x5f, // body
            ]
            .as_slice()
        );
    }

    #[test]
    fn test_header_decode_invalid_magic_cookie_value() {
        let mut input = [
            0x42, 0xdf, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ]
        .as_slice()
        .into();
        let err = decode_header(&mut input).unwrap_err();
        assert_matches!(err, DecodeError::InvalidMagicCookieValue(0x42dfad42));
    }

    #[test]
    fn test_header_decode_invalid_type_value() {
        let mut input = [
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 12, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00, // address
        ]
        .as_slice()
        .into();
        let err = decode_header(&mut input).unwrap_err();
        assert_matches!(err, DecodeError::InvalidTypeValue(12));
    }

    #[test]
    fn test_header_decode_invalid_flags_value() {
        let mut input = [
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x03, 13, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00, // address
        ]
        .as_slice()
        .into();
        let err = decode_header(&mut input).unwrap_err();
        assert_matches!(err, DecodeError::InvalidFlagsValue(13));
    }

    #[test]
    fn test_header_decode_unsupported_version() {
        let mut input = [
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x12, 0x34, 0x03, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00, // address
        ]
        .as_slice()
        .into();
        let err = decode_header(&mut input).unwrap_err();
        assert_matches!(err, DecodeError::UnsupportedVersion(Version(0x3412)));
    }
}
