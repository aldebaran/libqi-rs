//! Module defining `qi` the common binary representation of messages.
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
//! ║ │                         payload size                          │ ║
//! ║ ├───────────────────────────────┬───────────────┬───────────────┤ ║
//! ║ │            version            │     type      │    flags      │ ║
//! ║ ├───────────────────────────────┴───────────────┴───────────────┤ ║
//! ║ │                            service                            │ ║
//! ║ ├───────────────────────────────────────────────────────────────┤ ║
//! ║ │                            object                             │ ║
//! ║ ├───────────────────────────────────────────────────────────────┤ ║
//! ║ │                            action                             │ ║
//! ╠═╧═══════════════════════════════════════════════════════════════╧═╣
//! ║                             PAYLOAD                               ║
//! ╚═══════════════════════════════════════════════════════════════════╝
//! ```
//!
//! ### Header fields
//!  - magic cookie: 4 bytes, 0x42dead42 as big endian
//!  - id: 4 bytes unsigned integer, little endian
//!  - size/len: 4 bytes unsigned integer, size of the payload. may be 0, little endian
//!  - version: 2 bytes unsigned integer, little endian
//!  - type: 1 byte unsigned integer
//!  - flags: 1 byte unsigned integer
//!  - service: 4 bytes unsigned integer, little endian
//!  - object: 4 bytes unsigned integer, little endian
//!  - action: 4 bytes unsigned integer, little endian
//!
//!  The total header size is therefore 28 bytes.

use bytes::{Buf, BufMut};

macro_rules! define_message_newtype {
    ($vis:vis $name:ident($t:ty): $read:tt -> $readerr:ident, $write:tt) => {
        #[derive(
            Default,
            Debug,
            Hash,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Clone,
            Copy,
            derive_more::From,
            derive_more::Into,
            derive_more::Display,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        $vis struct $name($t);

        impl $name {
            const SIZE: usize = std::mem::size_of::<$t>();

            #[allow(dead_code)]
            $vis const fn new(val: $t) -> Self {
                Self(val)
            }

            #[allow(dead_code)]
            $vis const fn from(val: $t) -> Self {
                Self(val)
            }

            fn read<B>(buf: &mut B) -> Result<Self, $readerr>
            where
                B: Buf,
            {
                Ok(Self($read(buf)?))
            }

            fn write<B>(self, buf: &mut B)
            where
                B: BufMut,
            {
                buf.$write(self.0)
            }
        }

        #[derive(
            Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error,
        )]
        #[error(transparent)]
        struct $readerr(#[from] EndOfInputError);
    };
}

define_message_newtype!(pub(crate) Id(u32): read_u32_le -> IdReadError, put_u32_le);
define_message_newtype!(Version(u16): read_u16_le -> VersionReadError, put_u16_le);
define_message_newtype!(pub Service(u32): read_u32_le -> ServiceReadError, put_u32_le);
define_message_newtype!(pub Object(u32): read_u32_le -> ObjectReadError, put_u32_le);
define_message_newtype!(pub Action(u32): read_u32_le -> ActionReadError, put_u32_le);

impl Id {
    pub fn increment(&mut self) -> Id {
        let id = &mut self.0;
        *id = id.wrapping_add(1);
        Self(*id)
    }
}

impl Version {
    const CURRENT: Self = Self(0);
}

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
#[display(fmt = "action:{action}@object:{object}@service:{service}")]
pub struct Recipient {
    pub service: Service,
    pub object: Object,
    pub action: Action,
}

#[derive(
    Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, derive_more::Display,
)]
#[display(fmt = "{:#x}", "Self::VALUE")]
pub(crate) struct MagicCookie;

impl MagicCookie {
    pub(crate) const SIZE: usize = std::mem::size_of::<u32>();
    const VALUE: u32 = 0x42dead42;

    fn read<B>(buf: &mut B) -> Result<Self, MagicCookieReadError>
    where
        B: Buf,
    {
        let value = read_u32_be(buf)?;
        if value == Self::VALUE {
            Ok(MagicCookie)
        } else {
            Err(MagicCookieReadError::InvalidValue(value))
        }
    }

    fn write<B>(self, buf: &mut B)
    where
        B: BufMut,
    {
        buf.put_u32(Self::VALUE)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
enum MagicCookieReadError {
    #[error("invalid message magic cookie value {0:x}")]
    InvalidValue(u32),

    #[error(transparent)]
    EndOfInput(#[from] EndOfInputError),
}

#[derive(
    Default,
    Debug,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    derive_more::From,
    derive_more::Into,
    derive_more::Display,
)]
struct PayloadSize(usize);

impl PayloadSize {
    const SIZE: usize = std::mem::size_of::<u32>();

    fn read<B>(buf: &mut B) -> Result<Self, PayloadSizeReadError>
    where
        B: Buf,
    {
        let size = read_u32_le(buf)?;
        if size > (usize::MAX as u32) {
            return Err(PayloadSizeReadError::CannotBeRepresentedAsUSize(size));
        }
        let size = size as usize;
        Ok(Self(size))
    }

    fn write<B>(self, buf: &mut B) -> Result<(), PayloadSizeWriteError>
    where
        B: BufMut,
    {
        let size = self.0;
        if size > (u32::MAX as usize) {
            return Err(PayloadSizeWriteError::CannotBeRepresentedAsU32(size));
        }
        let size = size as u32;
        buf.put_u32_le(size);
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
enum PayloadSizeReadError {
    #[error(
        "message payload size {0} cannot be represented as an usize (the maximum for this system is {})",
        usize::MAX
    )]
    CannotBeRepresentedAsUSize(u32),

    #[error(transparent)]
    EndOfInput(#[from] EndOfInputError),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
enum PayloadSizeWriteError {
    #[error(
        "message payload size {0} cannot be represented as an u32 (the maximum for this system is {})",
        u32::MAX
    )]
    CannotBeRepresentedAsU32(usize),
}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    derive_more::Display,
    num_derive::FromPrimitive,
    num_derive::ToPrimitive,
)]
#[repr(u8)]
pub(crate) enum Type {
    #[display(fmt = "call")]
    Call = 1,
    #[display(fmt = "reply")]
    Reply = 2,
    #[display(fmt = "error")]
    Error = 3,
    #[display(fmt = "post")]
    Post = 4,
    #[display(fmt = "event")]
    Event = 5,
    #[display(fmt = "capabilities")]
    Capabilities = 6,
    #[display(fmt = "cancel")]
    Cancel = 7,
    #[display(fmt = "canceled")]
    Canceled = 8,
}

impl Type {
    const SIZE: usize = std::mem::size_of::<u8>();

    fn read<B>(buf: &mut B) -> Result<Self, TypeReadError>
    where
        B: Buf,
    {
        let ty = read_u8(buf)?;
        let ty = Type::try_from(ty).map_err(|err| TypeReadError::InvalidValue(err.0))?;
        Ok(ty)
    }

    fn write<B>(self, buf: &mut B)
    where
        B: BufMut,
    {
        buf.put_u8(self.into())
    }
}

impl Default for Type {
    fn default() -> Self {
        Self::Call
    }
}

impl From<Type> for u8 {
    fn from(ty: Type) -> u8 {
        use num_traits::ToPrimitive;
        ty.to_u8().unwrap()
    }
}

impl std::convert::TryFrom<u8> for Type {
    type Error = InvalidTypeValueError;

    fn try_from(value: u8) -> Result<Self, InvalidTypeValueError> {
        use num_traits::FromPrimitive;
        Self::from_u8(value).ok_or(InvalidTypeValueError(value))
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, thiserror::Error)]
#[error("invalid message type value {0}")]
pub(crate) struct InvalidTypeValueError(u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
enum TypeReadError {
    #[error("{}", InvalidTypeValueError(*.0))]
    InvalidValue(u8),

    #[error(transparent)]
    EndOfInput(#[from] EndOfInputError),
}

bitflags::bitflags! {
    #[derive(Default, derive_more::Display)]
    #[display(fmt = "{:b}", "self.bits()")]
    pub(crate) struct Flags: u8 {
        const DYNAMIC_PAYLOAD = 0b00000001;
        const RETURN_TYPE = 0b00000010;
    }
}

impl Flags {
    const SIZE: usize = std::mem::size_of::<u8>();

    fn read<B>(buf: &mut B) -> Result<Self, FlagsReadError>
    where
        B: Buf,
    {
        let byte = read_u8(buf)?;
        let flags = Self::try_from(byte).map_err(|err| FlagsReadError::InvalidValue(err.0))?;
        Ok(flags)
    }

    fn write<B>(self, buf: &mut B)
    where
        B: BufMut,
    {
        buf.put_u8(self.bits())
    }
}

impl std::convert::TryFrom<u8> for Flags {
    type Error = InvalidFlagsValueError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value).ok_or(InvalidFlagsValueError(value))
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
#[error("invalid message flags value {0}")]
pub(crate) struct InvalidFlagsValueError(u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
enum FlagsReadError {
    #[error("{}", InvalidFlagsValueError(*.0))]
    InvalidValue(u8),

    #[error(transparent)]
    EndOfInput(#[from] EndOfInputError),
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub(crate) struct Header {
    id: Id,
    ty: Type,
    payload_size: PayloadSize,
    flags: Flags,
    service: Service,
    object: Object,
    action: Action,
}

impl Header {
    const MAGIC_COOKIE_OFFSET: usize = 0;
    const ID_OFFSET: usize = Self::MAGIC_COOKIE_OFFSET + MagicCookie::SIZE;
    const PAYLOAD_SIZE_OFFSET: usize = Self::ID_OFFSET + Id::SIZE;
    const VERSION_OFFSET: usize = Self::PAYLOAD_SIZE_OFFSET + PayloadSize::SIZE;
    const TYPE_OFFSET: usize = Self::VERSION_OFFSET + Version::SIZE;
    const FLAGS_OFFSET: usize = Self::TYPE_OFFSET + Type::SIZE;
    const SERVICE_OFFSET: usize = Self::FLAGS_OFFSET + Flags::SIZE;
    const OBJECT_OFFSET: usize = Self::SERVICE_OFFSET + Service::SIZE;
    const ACTION_OFFSET: usize = Self::OBJECT_OFFSET + Object::SIZE;
    pub(crate) const SIZE: usize = Self::ACTION_OFFSET + Action::SIZE;

    pub(crate) fn read<B>(buf: &mut B) -> Result<Self, HeaderReadError>
    where
        B: Buf,
    {
        MagicCookie::read(buf)?;
        let id = Id::read(buf)?;
        let payload_size = PayloadSize::read(buf)?;
        let version = Version::read(buf)?;
        if version != Version::CURRENT {
            return Err(HeaderReadError::UnsupportedVersion(version.into()));
        }
        let ty = Type::read(buf)?;
        let flags = Flags::read(buf)?;
        let service = Service::read(buf)?;
        let object = Object::read(buf)?;
        let action = Action::read(buf)?;
        Ok(Self {
            id,
            ty,
            payload_size,
            flags,
            service,
            object,
            action,
        })
    }

    pub(crate) fn write<B>(self, buf: &mut B) -> Result<(), HeaderWriteError>
    where
        B: BufMut,
    {
        let mut hbuf = [0u8; Header::SIZE];
        let mut hbuf_ref = hbuf.as_mut();
        MagicCookie.write(&mut hbuf_ref);
        self.id.write(&mut hbuf_ref);
        self.payload_size.write(&mut hbuf_ref)?;
        Version::CURRENT.write(&mut hbuf_ref);
        self.ty.write(&mut hbuf_ref);
        self.flags.write(&mut hbuf_ref);
        self.service.write(&mut hbuf_ref);
        self.object.write(&mut hbuf_ref);
        self.action.write(&mut hbuf_ref);
        buf.put(hbuf.as_ref());
        Ok(())
    }

    pub(crate) fn payload_size(&self) -> usize {
        self.payload_size.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub(crate) enum HeaderReadError {
    #[error(transparent)]
    EndOfInput(#[from] EndOfInputError),

    #[error("{}", MagicCookieReadError::InvalidValue(*.0))]
    InvalidMessageCookieValue(u32),

    #[error("{}", PayloadSizeReadError::CannotBeRepresentedAsUSize(*.0))]
    PayloadSizeCannotBeRepresentedAsUSize(u32),

    #[error("unsupported message version {0}")]
    UnsupportedVersion(u16),

    #[error("{}", TypeReadError::InvalidValue(*.0))]
    InvalidTypeValue(u8),

    #[error("{}", FlagsReadError::InvalidValue(*.0))]
    InvalidFlagsValue(u8),
}

impl From<MagicCookieReadError> for HeaderReadError {
    fn from(e: MagicCookieReadError) -> Self {
        match e {
            MagicCookieReadError::InvalidValue(v) => Self::InvalidMessageCookieValue(v),
            MagicCookieReadError::EndOfInput(EndOfInputError { actual, .. }) => {
                Self::EndOfInput(EndOfInputError {
                    expected: Header::SIZE,
                    actual,
                })
            }
        }
    }
}

impl From<IdReadError> for HeaderReadError {
    fn from(e: IdReadError) -> Self {
        let IdReadError(EndOfInputError { actual, .. }) = e;
        Self::EndOfInput(EndOfInputError {
            expected: Header::SIZE,
            actual: Header::ID_OFFSET + actual,
        })
    }
}

impl From<PayloadSizeReadError> for HeaderReadError {
    fn from(e: PayloadSizeReadError) -> Self {
        match e {
            PayloadSizeReadError::CannotBeRepresentedAsUSize(s) => {
                Self::PayloadSizeCannotBeRepresentedAsUSize(s)
            }
            PayloadSizeReadError::EndOfInput(EndOfInputError { actual, .. }) => {
                Self::EndOfInput(EndOfInputError {
                    expected: Header::SIZE,
                    actual: Header::PAYLOAD_SIZE_OFFSET + actual,
                })
            }
        }
    }
}

impl From<VersionReadError> for HeaderReadError {
    fn from(e: VersionReadError) -> Self {
        let VersionReadError(EndOfInputError { actual, .. }) = e;
        Self::EndOfInput(EndOfInputError {
            expected: Header::SIZE,
            actual: Header::VERSION_OFFSET + actual,
        })
    }
}

impl From<TypeReadError> for HeaderReadError {
    fn from(e: TypeReadError) -> Self {
        match e {
            TypeReadError::InvalidValue(v) => Self::InvalidTypeValue(v),
            TypeReadError::EndOfInput(EndOfInputError { actual, .. }) => {
                Self::EndOfInput(EndOfInputError {
                    expected: Header::SIZE,
                    actual: Header::TYPE_OFFSET + actual,
                })
            }
        }
    }
}

impl From<FlagsReadError> for HeaderReadError {
    fn from(e: FlagsReadError) -> Self {
        match e {
            FlagsReadError::InvalidValue(v) => Self::InvalidFlagsValue(v),
            FlagsReadError::EndOfInput(EndOfInputError { actual, .. }) => {
                Self::EndOfInput(EndOfInputError {
                    expected: Header::SIZE,
                    actual: Header::FLAGS_OFFSET + actual,
                })
            }
        }
    }
}

impl From<ServiceReadError> for HeaderReadError {
    fn from(e: ServiceReadError) -> Self {
        let ServiceReadError(EndOfInputError { actual, .. }) = e;
        Self::EndOfInput(EndOfInputError {
            expected: Header::SIZE,
            actual: Header::SERVICE_OFFSET + actual,
        })
    }
}

impl From<ObjectReadError> for HeaderReadError {
    fn from(e: ObjectReadError) -> Self {
        let ObjectReadError(EndOfInputError { actual, .. }) = e;
        Self::EndOfInput(EndOfInputError {
            expected: Header::SIZE,
            actual: Header::OBJECT_OFFSET + actual,
        })
    }
}

impl From<ActionReadError> for HeaderReadError {
    fn from(e: ActionReadError) -> Self {
        let ActionReadError(EndOfInputError { actual, .. }) = e;
        Self::EndOfInput(EndOfInputError {
            expected: Header::SIZE,
            actual: Header::ACTION_OFFSET + actual,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub(crate) enum HeaderWriteError {
    #[error("{}", PayloadSizeWriteError::CannotBeRepresentedAsU32(*.0))]
    PayloadSizeCannotBeRepresentedAsU32(usize),
}

impl From<PayloadSizeWriteError> for HeaderWriteError {
    fn from(e: PayloadSizeWriteError) -> Self {
        match e {
            PayloadSizeWriteError::CannotBeRepresentedAsU32(v) => {
                Self::PayloadSizeCannotBeRepresentedAsU32(v)
            }
        }
    }
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    derive_more::From,
    derive_more::Into,
)]
pub(crate) struct Payload(Vec<u8>);

impl Payload {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub(crate) fn read<B>(size: usize, buf: &mut B) -> Result<Self, PayloadReadError>
    where
        B: Buf,
    {
        let data_len = buf.remaining();
        if data_len < size {
            return Err(PayloadReadError(EndOfInputError {
                expected: size,
                actual: data_len,
            }));
        }
        let mut payload = vec![0; size];
        buf.copy_to_slice(payload.as_mut_slice());
        Ok(Self(payload))
    }

    fn write<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        buf.put_slice(&self.0);
    }

    pub(crate) fn size(&self) -> usize {
        self.0.len()
    }
}

impl AsRef<[u8]> for Payload {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
#[error(transparent)]
pub(crate) struct PayloadReadError(#[from] pub(crate) EndOfInputError);

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, derive_more::Display)]
#[display(fmt = "message(id={id}, type={ty}, flags={flags}, recipient={recipient})")]
pub(crate) struct Message {
    pub(crate) id: Id,
    pub(crate) ty: Type,
    pub(crate) flags: Flags,
    pub(crate) recipient: Recipient,
    pub(crate) payload: Payload,
}

impl Message {
    pub(crate) fn new(header: Header, payload: Payload) -> Self {
        Self {
            id: header.id,
            ty: header.ty,
            flags: header.flags,
            recipient: Recipient {
                service: header.service,
                object: header.object,
                action: header.action,
            },
            payload,
        }
    }

    pub(crate) fn write<B>(self, buf: &mut B) -> Result<(), HeaderWriteError>
    where
        B: BufMut,
    {
        Header {
            id: self.id,
            ty: self.ty,
            payload_size: PayloadSize(self.payload.size()),
            flags: self.flags,
            service: self.recipient.service,
            object: self.recipient.object,
            action: self.recipient.action,
        }
        .write(buf)?;
        self.payload.write(buf);
        Ok(())
    }

    pub(crate) fn size(&self) -> usize {
        Header::SIZE + self.payload.size()
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
#[error("end of input: not enough data to read value, expected {expected} bytes but only got {actual} bytes")]
pub(crate) struct EndOfInputError {
    pub(crate) expected: usize,
    pub(crate) actual: usize,
}

fn read<B, F, T>(buf: &mut B, read_fn: F) -> Result<T, EndOfInputError>
where
    B: Buf,
    F: FnOnce(&mut B) -> T,
{
    let value_size = std::mem::size_of::<T>();
    let data_len = buf.remaining();
    if data_len < value_size {
        return Err(EndOfInputError {
            expected: value_size,
            actual: data_len,
        });
    }
    let value = read_fn(buf);
    Ok(value)
}

fn read_u8<B>(buf: &mut B) -> Result<u8, EndOfInputError>
where
    B: Buf,
{
    read(buf, Buf::get_u8)
}

fn read_u16_le<B>(buf: &mut B) -> Result<u16, EndOfInputError>
where
    B: Buf,
{
    read(buf, Buf::get_u16_le)
}

fn read_u32_be<B>(buf: &mut B) -> Result<u32, EndOfInputError>
where
    B: Buf,
{
    read(buf, Buf::get_u32)
}

fn read_u32_le<B>(buf: &mut B) -> Result<u32, EndOfInputError>
where
    B: Buf,
{
    read(buf, Buf::get_u32_le)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_header_size() {
        assert_eq!(Header::SIZE, 28);
    }

    #[test]
    fn test_message_read() {
        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            // payload
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let header = Header::read(&mut input).unwrap();
        let payload = Payload::read(header.payload_size(), &mut input).unwrap();
        let message = Message::new(header, payload);
        assert_eq!(
            message,
            Message {
                id: Id::new(990340),
                ty: Type::Error,
                flags: Flags::empty(),
                recipient: Recipient {
                    service: Service::new(47),
                    object: Object::new(1),
                    action: Action::new(178),
                },
                payload: Payload::new(vec![
                    0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20,
                    0x72, 0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20,
                    0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64
                ]),
            }
        );
    }

    #[test]
    fn test_message_write() {
        use crate::message::*;
        let msg = Message {
            id: Id(329),
            ty: Type::Capabilities,
            flags: Flags::RETURN_TYPE,
            recipient: Recipient {
                service: Service(1),
                object: Object(1),
                action: Action(104),
            },
            payload: Payload(vec![0x17, 0x2b, 0xe6, 0x01, 0x5f]),
        };
        let mut buf = Vec::new();
        msg.write(&mut buf).unwrap();

        assert_eq!(
            buf,
            [
                0x42, 0xde, 0xad, 0x42, // cookie
                0x49, 0x01, 0x00, 0x00, // id
                0x05, 0x00, 0x00, 0x00, // size
                0x00, 0x00, 0x06, 0x02, // version, type, flags
                0x01, 0x00, 0x00, 0x00, // service
                0x01, 0x00, 0x00, 0x00, // object
                0x68, 0x00, 0x00, 0x00, // action
                0x17, 0x2b, 0xe6, 0x01, 0x5f, // payload
            ]
        );
    }

    #[test]
    fn test_header_read_invalid_magic_cookie_value() {
        let mut input: &[u8] = &[
            0x42, 0xdf, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let header = Header::read(&mut input);
        assert_eq!(
            header,
            Err(HeaderReadError::InvalidMessageCookieValue(0x42dfad42))
        );
    }

    #[test]
    fn test_header_read_invalid_type_value() {
        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0xaa, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0xb2, 0x00, 0x00, 0x00, // action
        ];
        let header = Header::read(&mut input);
        assert_eq!(header, Err(HeaderReadError::InvalidTypeValue(0xaa)));
    }

    #[test]
    fn test_header_read_invalid_flags_value() {
        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x03, 0x13, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0xb2, 0x00, 0x00, 0x00, // action
        ];
        let header = Header::read(&mut input);
        assert_eq!(header, Err(HeaderReadError::InvalidFlagsValue(0x13)));
    }

    #[test]
    fn test_header_read_unsupported_version() {
        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x12, 0x34, 0x03, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0xb2, 0x00, 0x00, 0x00, // action
        ];
        let header = Header::read(&mut input);
        assert_eq!(header, Err(HeaderReadError::UnsupportedVersion(0x3412)));
    }

    #[test]
    fn test_message_read_not_enough_data() {
        fn check_header(mut input: &[u8], actual: usize) {
            let header = Header::read(&mut input);
            assert_eq!(
                header,
                Err(HeaderReadError::EndOfInput(EndOfInputError {
                    expected: 28,
                    actual
                }))
            );
        }

        check_header(
            &[
                0x42, 0xde, 0xad, // cookie, 1 byte short
            ],
            3,
        );

        check_header(
            &[
                0x42, 0xde, 0xad, 0x42, // cookie,
                0x84, 0x1c, 0x0f, // id, 1 byte short
            ],
            7,
        );

        check_header(
            &[
                0x42, 0xde, 0xad, 0x42, // cookie,
                0x84, 0x1c, 0x0f, 0x00, // id
                0x23, 0x00, 0x00, // size, 1 byte short
            ],
            11,
        );

        check_header(
            &[
                0x42, 0xde, 0xad, 0x42, // cookie,
                0x84, 0x1c, 0x0f, 0x00, // id
                0x23, 0x00, 0x00, 0x00, // size
                0x00, // version 1 byte short
            ],
            13,
        );

        check_header(
            &[
                0x42, 0xde, 0xad, 0x42, // cookie,
                0x84, 0x1c, 0x0f, 0x00, // id
                0x23, 0x00, 0x00, 0x00, // size
                0x00, 0x00, // version, type 1 byte short
            ],
            14,
        );

        check_header(
            &[
                0x42, 0xde, 0xad, 0x42, // cookie,
                0x84, 0x1c, 0x0f, 0x00, // id
                0x23, 0x00, 0x00, 0x00, // size
                0x00, 0x00, 0x03, // version, type, flags 1 byte short
            ],
            15,
        );

        check_header(
            &[
                0x42, 0xde, 0xad, 0x42, // cookie,
                0x84, 0x1c, 0x0f, 0x00, // id
                0x23, 0x00, 0x00, 0x00, // size
                0x00, 0x00, 0x03, 0x00, // version, type, flags
                0x2f, 0x00, 0x00, // service, 1 byte short
            ],
            19,
        );

        check_header(
            &[
                0x42, 0xde, 0xad, 0x42, // cookie,
                0x84, 0x1c, 0x0f, 0x00, // id
                0x23, 0x00, 0x00, 0x00, // size
                0x00, 0x00, 0x03, 0x00, // version, type, flags
                0x2f, 0x00, 0x00, 0x00, // service
                0x01, 0x00, 0x00, // object, 1 byte short
            ],
            23,
        );

        check_header(
            &[
                0x42, 0xde, 0xad, 0x42, // cookie,
                0x84, 0x1c, 0x0f, 0x00, // id
                0x23, 0x00, 0x00, 0x00, // size
                0x00, 0x00, 0x03, 0x00, // version, type, flags
                0x2f, 0x00, 0x00, 0x00, // service
                0x01, 0x00, 0x00, 0x00, // object
                0xb2, 0x00, 0x00, // action, 1 byte short
            ],
            27,
        );

        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x04, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x03, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0xb2, 0x00, 0x00, 0x00, // action
            0x01, 0x02, 0x03, // payload, 1 byte short
        ];
        let header = Header::read(&mut input).unwrap();
        let payload = Payload::read(header.payload_size(), &mut input);
        assert_eq!(
            payload,
            Err(PayloadReadError(EndOfInputError {
                expected: 4,
                actual: 3
            }))
        );
    }
}
