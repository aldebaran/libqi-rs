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
    body::BodyBuf,
    message::{Address, Id, Message, MetaData, Type, Version},
};
use bytes::{Buf, BufMut, BytesMut};
use std::marker::PhantomData;
use tracing::instrument;

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub struct Encoder;

impl<T> tokio_util::codec::Encoder<Message<T>> for Encoder
where
    T: BodyBuf,
{
    type Error = EncodeError<T::Error>;

    #[instrument(level = "trace", skip_all, err)]
    fn encode(&mut self, msg: Message<T>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (meta, body) = msg.into_parts().map_err(EncodeError::BodyConversion)?;
        let body_data = body.into_data().map_err(EncodeError::BodyConversion)?;
        let body_size = body_data.remaining();
        let msg_size = HEADER_SIZE + body_size;
        dst.reserve(msg_size);
        put_header(meta, body_size, dst)?;
        dst.put(body_data);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError<E> {
    #[error(
        "message body size {0} cannot be represented as an u32 (the maximum for this system is {})",
        u32::MAX
    )]
    BodySizeCannotBeRepresentedAsU32Error(usize),

    #[error("body conversion error")]
    BodyConversion(#[source] E),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub struct Decoder<'a, T> {
    state: DecoderState,
    phantom: PhantomData<fn() -> &'a T>,
}

impl<'a, T> Decoder<'a, T>
where
    T: BodyBuf,
{
    pub fn new() -> Self {
        Self {
            state: DecoderState::default(),
            phantom: PhantomData,
        }
    }

    fn decode_body(
        &mut self,
        meta: &MetaData,
        body_size: usize,
        src: &mut BytesMut,
    ) -> Result<Option<Message<T>>, DecodeError<T::Error>> {
        if src.len() < body_size {
            src.reserve(body_size - src.len());
            return Ok(None);
        }
        let body_bytes = src.split_to(body_size).freeze();
        let message = T::from_bytes(body_bytes)
            .and_then(|body| Message::from_parts(*meta, body))
            .map_err(DecodeError::BodyConversion)?;
        Ok(Some(message))
    }
}

impl<'a, T> tokio_util::codec::Decoder for Decoder<'a, T>
where
    T: BodyBuf,
{
    type Item = Message<T>;
    type Error = DecodeError<T::Error>;

    #[instrument(level = "trace", skip_all, err)]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let msg = loop {
            match self.state {
                DecoderState::Header => match decode_header(src)? {
                    None => break None,
                    Some((body_size, meta)) => self.state = DecoderState::Body(body_size, meta),
                },
                DecoderState::Body(size, meta) => {
                    let msg = self.decode_body(&meta, size, src)?;
                    if msg.is_some() {
                        self.state = DecoderState::Header
                    }
                    break msg;
                }
            }
        };
        Ok(msg)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError<E> {
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

    #[error("body conversion error")]
    BodyConversion(#[source] E),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

fn decode_header<ErrDeserBody>(
    src: &mut BytesMut,
) -> Result<Option<(usize, MetaData)>, DecodeError<ErrDeserBody>> {
    if src.len() < HEADER_SIZE {
        src.reserve(HEADER_SIZE - src.len());
        return Ok(None);
    }

    get_magic_cookie(src)?;
    let id = get_id(src);
    let body_size = get_body_size(src)?;
    let version = get_version(src);

    // The only supported version of messages at the moment is the current.
    if version != Version::ZERO {
        return Err(DecodeError::UnsupportedVersion(version));
    }

    let ty = get_type(src)?;
    src.advance(1); // Flags
    let address = get_address(src);
    let meta = MetaData { id, address, ty };
    Ok(Some((body_size, meta)))
}

fn put_header<ErrSerBody>(
    meta: MetaData,
    body_size: usize,
    dst: &mut BytesMut,
) -> Result<(), EncodeError<ErrSerBody>> {
    put_magic_cookie(dst);
    put_id(meta.id, dst);
    put_body_size(body_size, dst)?;
    put_version(Version::ZERO, dst);
    put_type(meta.ty, dst);
    dst.put_u8(0); // Flags
    put_address(meta.address, dst);
    Ok(())
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
enum DecoderState {
    #[default]
    Header,
    Body(usize, MetaData),
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

fn get_magic_cookie<ErrDeserBody>(src: &mut BytesMut) -> Result<(), DecodeError<ErrDeserBody>> {
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

fn get_body_size<ErrDeserBody>(src: &mut BytesMut) -> Result<usize, DecodeError<ErrDeserBody>> {
    let size = src.get_u32_le();
    if size > (usize::MAX as u32) {
        return Err(DecodeError::BodySizeCannotBeRepresentedAsUSize(size));
    }
    let size = size as usize;
    Ok(size)
}

fn put_body_size<ErrSerBody>(
    size: usize,
    dst: &mut BytesMut,
) -> Result<(), EncodeError<ErrSerBody>> {
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

/// Type representation values:
///   - Call = 1
///   - Reply = 2
///   - Error = 3
///   - Post = 4
///   - Event = 5
///   - Capabilities = 6
///   - Cancel = 7
///   - Canceled = 8
fn get_type<ErrDeserBody>(src: &mut BytesMut) -> Result<Type, DecodeError<ErrDeserBody>> {
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

fn get_address(src: &mut BytesMut) -> Address {
    let service = src.get_u32_le().into();
    let object = src.get_u32_le().into();
    let action = src.get_u32_le().into();
    Address(service, object, action)
}

fn put_address(Address(service, object, action): Address, dst: &mut BytesMut) {
    dst.put_u32_le(service.into());
    dst.put_u32_le(object.into());
    dst.put_u32_le(action.into());
}
