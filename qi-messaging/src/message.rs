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

use crate::capabilities;
use bytes::{Buf, BufMut};
use qi_format::{from_bytes, to_bytes};

fn read<B, F, T>(buf: &mut B, read_fn: F) -> Result<T, NotEnoughDataError>
where
    B: Buf,
    F: FnOnce(&mut B) -> T,
{
    let value_size = std::mem::size_of::<T>();
    let data_len = buf.remaining();
    if data_len < value_size {
        return Err(NotEnoughDataError {
            expected: value_size,
            actual: data_len,
        });
    }
    let value = read_fn(buf);
    Ok(value)
}

fn read_u8<B>(buf: &mut B) -> Result<u8, NotEnoughDataError>
where
    B: Buf,
{
    read(buf, Buf::get_u8)
}

fn read_u16_le<B>(buf: &mut B) -> Result<u16, NotEnoughDataError>
where
    B: Buf,
{
    read(buf, Buf::get_u16_le)
}

fn read_u32_be<B>(buf: &mut B) -> Result<u32, NotEnoughDataError>
where
    B: Buf,
{
    read(buf, Buf::get_u32)
}

fn read_u32_le<B>(buf: &mut B) -> Result<u32, NotEnoughDataError>
where
    B: Buf,
{
    read(buf, Buf::get_u32_le)
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
#[error("not enough data to read value, expected {expected} bytes but only got {actual} bytes")]
pub struct NotEnoughDataError {
    pub expected: usize,
    pub actual: usize,
}

macro_rules! define_message_newtype {
    ($name:ident($t:ty): $read:tt -> $readerr:ident, $write:tt) => {
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
        pub struct $name($t);

        impl $name {
            const SIZE: usize = std::mem::size_of::<$t>();

            pub const fn new(val: $t) -> Self {
                Self(val)
            }

            const fn from(val: $t) -> Self {
                Self(val)
            }

            const fn into(self) -> $t {
                self.0
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
        pub struct $readerr(#[from] NotEnoughDataError);
    };
}

define_message_newtype!(Id(u32): read_u32_le -> IdReadError, put_u32_le);
define_message_newtype!(Version(u16): read_u16_le -> VersionReadError, put_u16_le);
define_message_newtype!(Service(u32): read_u32_le -> ServiceReadError, put_u32_le);
define_message_newtype!(Object(u32): read_u32_le -> ObjectReadError, put_u32_le);
define_message_newtype!(Action(u32): read_u32_le -> ActionReadError, put_u32_le);

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

impl Service {
    const SERVER: Self = Self(0);
}

#[derive(
    Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, derive_more::Display,
)]
#[display(fmt = "{:#x}", "Self::VALUE")]
struct MagicCookie;

impl MagicCookie {
    const SIZE: usize = std::mem::size_of::<u32>();
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
    NotEnoughData(#[from] NotEnoughDataError),
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

    const fn into(self) -> usize {
        self.0
    }

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
    NotEnoughData(#[from] NotEnoughDataError),
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
enum Type {
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
    #[display(fmt = "capability")]
    Capability = 6,
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
struct InvalidTypeValueError(u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
enum TypeReadError {
    #[error("{}", InvalidTypeValueError(*.0))]
    InvalidValue(u8),

    #[error(transparent)]
    NotEnoughData(#[from] NotEnoughDataError),
}

bitflags::bitflags! {
    #[derive(Default, derive_more::Display)]
    #[display(fmt = "{:b}", "self.bits()")]
    struct Flags: u8 {
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
struct InvalidFlagsValueError(u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
enum FlagsReadError {
    #[error("{}", InvalidFlagsValueError(*.0))]
    InvalidValue(u8),

    #[error(transparent)]
    NotEnoughData(#[from] NotEnoughDataError),
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct Header {
    pub id: Id,
    pub ty: Type,
    pub payload_size: PayloadSize,
    pub flags: Flags,
    pub service: Service,
    pub object: Object,
    pub action: Action,
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
    const SIZE: usize = Self::ACTION_OFFSET + Action::SIZE;

    fn read<B>(buf: &mut B) -> Result<Self, HeaderReadError>
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

    fn write<B>(self, buf: &mut B) -> Result<(), HeaderWriteError>
    where
        B: BufMut,
    {
        MagicCookie.write(buf);
        self.id.write(buf);
        self.payload_size.write(buf)?;
        Version::CURRENT.write(buf);
        self.ty.write(buf);
        self.flags.write(buf);
        self.service.write(buf);
        self.object.write(buf);
        self.action.write(buf);
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub enum HeaderReadError {
    #[error(transparent)]
    NotEnoughData(#[from] NotEnoughDataError),

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
            MagicCookieReadError::NotEnoughData(NotEnoughDataError { actual, .. }) => {
                Self::NotEnoughData(NotEnoughDataError {
                    expected: Header::SIZE,
                    actual,
                })
            }
        }
    }
}

impl From<IdReadError> for HeaderReadError {
    fn from(e: IdReadError) -> Self {
        let IdReadError(NotEnoughDataError { actual, .. }) = e;
        Self::NotEnoughData(NotEnoughDataError {
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
            PayloadSizeReadError::NotEnoughData(NotEnoughDataError { actual, .. }) => {
                Self::NotEnoughData(NotEnoughDataError {
                    expected: Header::SIZE,
                    actual: Header::PAYLOAD_SIZE_OFFSET + actual,
                })
            }
        }
    }
}

impl From<VersionReadError> for HeaderReadError {
    fn from(e: VersionReadError) -> Self {
        let VersionReadError(NotEnoughDataError { actual, .. }) = e;
        Self::NotEnoughData(NotEnoughDataError {
            expected: Header::SIZE,
            actual: Header::VERSION_OFFSET + actual,
        })
    }
}

impl From<TypeReadError> for HeaderReadError {
    fn from(e: TypeReadError) -> Self {
        match e {
            TypeReadError::InvalidValue(v) => Self::InvalidTypeValue(v),
            TypeReadError::NotEnoughData(NotEnoughDataError { actual, .. }) => {
                Self::NotEnoughData(NotEnoughDataError {
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
            FlagsReadError::NotEnoughData(NotEnoughDataError { actual, .. }) => {
                Self::NotEnoughData(NotEnoughDataError {
                    expected: Header::SIZE,
                    actual: Header::FLAGS_OFFSET + actual,
                })
            }
        }
    }
}

impl From<ServiceReadError> for HeaderReadError {
    fn from(e: ServiceReadError) -> Self {
        let ServiceReadError(NotEnoughDataError { actual, .. }) = e;
        Self::NotEnoughData(NotEnoughDataError {
            expected: Header::SIZE,
            actual: Header::SERVICE_OFFSET + actual,
        })
    }
}

impl From<ObjectReadError> for HeaderReadError {
    fn from(e: ObjectReadError) -> Self {
        let ObjectReadError(NotEnoughDataError { actual, .. }) = e;
        Self::NotEnoughData(NotEnoughDataError {
            expected: Header::SIZE,
            actual: Header::OBJECT_OFFSET + actual,
        })
    }
}

impl From<ActionReadError> for HeaderReadError {
    fn from(e: ActionReadError) -> Self {
        let ActionReadError(NotEnoughDataError { actual, .. }) = e;
        Self::NotEnoughData(NotEnoughDataError {
            expected: Header::SIZE,
            actual: Header::ACTION_OFFSET + actual,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub enum HeaderWriteError {
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

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct Payload(Vec<u8>);

impl Payload {
    fn read<B>(size: usize, buf: &mut B) -> Result<Self, PayloadReadError>
    where
        B: Buf,
    {
        let data_len = buf.remaining();
        if data_len < size {
            return Err(PayloadReadError(NotEnoughDataError {
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

    fn size(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
#[error(transparent)]
pub struct PayloadReadError(#[from] pub NotEnoughDataError);

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, derive_more::Display)]
#[display(
    fmt = "message(id={id}, type={ty}, flags={flags}, service={service}, object={object}, action={action})"
)]
pub struct Message {
    id: Id,
    ty: Type,
    flags: Flags,
    service: Service,
    object: Object,
    action: Action,
    payload: Payload,
}

impl Message {
    pub fn read<B>(buf: &mut B) -> Result<Self, ReadError>
    where
        B: Buf,
    {
        let header = Header::read(buf)?;
        let payload = Payload::read(header.payload_size.into(), buf)?;

        Ok(Self {
            id: header.id,
            ty: header.ty,
            flags: header.flags,
            service: header.service,
            object: header.object,
            action: header.action,
            payload,
        })
    }

    pub fn write<B>(self, buf: &mut B) -> Result<(), HeaderWriteError>
    where
        B: BufMut,
    {
        Header {
            id: self.id,
            ty: self.ty,
            payload_size: PayloadSize(self.payload.size()),
            flags: self.flags,
            service: self.service,
            object: self.object,
            action: self.action,
        }
        .write(buf)?;
        self.payload.write(buf);
        Ok(())
    }

    pub fn size(&self) -> usize {
        Header::SIZE + self.payload.size()
    }

    pub fn into_capability(self) -> Result<Capability, IntoCapabilityError> {
        match self.ty {
            Type::Capability => Ok(Capability {
                id: self.id,
                capabilities: from_bytes(&self.payload.0)?,
            }),
            _ => Err(IntoCapabilityError::BadType(self)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub enum ReadError {
    #[error("error reading message header: {0}")]
    Header(#[from] HeaderReadError),

    #[error("error reading message payload: {0}")]
    Payload(#[from] PayloadReadError),
}

#[derive(thiserror::Error, Debug)]
pub enum IntoMessageError {
    #[error("serialization error: {0}")]
    SerializationError(#[from] qi_format::Error),
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Call {
    pub id: Id,
    pub dynamic_payload: bool,
    pub return_type: bool,
    pub service: Service,
    pub object: Object,
    pub action: Action,
    pub payload: Vec<u8>,
}

impl Call {
    pub fn builder(id: Id) -> CallBuilder {
        CallBuilder {
            call: Self {
                id,
                ..Default::default()
            },
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct CallBuilder {
    call: Call,
}

impl CallBuilder {
    pub fn dynamic_payload(mut self, value: bool) -> Self {
        self.call.dynamic_payload = value;
        self
    }

    pub fn return_type(mut self, value: bool) -> Self {
        self.call.return_type = value;
        self
    }

    pub fn service(mut self, value: Service) -> Self {
        self.call.service = value;
        self
    }

    pub fn object(mut self, value: Object) -> Self {
        self.call.object = value;
        self
    }

    pub fn action(mut self, value: Action) -> Self {
        self.call.action = value;
        self
    }

    pub fn payload(mut self, value: Vec<u8>) -> Self {
        self.call.payload = value;
        self
    }

    pub fn build(self) -> Call {
        self.call
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Reply {
    pub id: Id,
    pub dynamic_payload: bool,
    pub service: Service,
    pub object: Object,
    pub action: Action,
    pub payload: Vec<u8>,
}

impl Reply {
    pub fn builder_to(call: &Call) -> ReplyBuilder {
        ReplyBuilder {
            reply: Self {
                id: call.id,
                service: call.service,
                object: call.object,
                action: call.action,
                ..Default::default()
            },
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct ReplyBuilder {
    reply: Reply,
}

impl ReplyBuilder {
    pub fn dynamic_payload(mut self, value: bool) -> Self {
        self.reply.dynamic_payload = value;
        self
    }

    pub fn payload(mut self, value: Vec<u8>) -> Self {
        self.reply.payload = value;
        self
    }

    pub fn build(self) -> Reply {
        self.reply
    }
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct Capability {
    pub id: Id,
    pub capabilities: capabilities::CapabilityMap,
}

impl Capability {
    pub fn new(id: Id, capabilities: capabilities::CapabilityMap) -> Self {
        Self { id, capabilities }
    }

    pub fn into_message(self) -> Result<Message, IntoMessageError> {
        Ok(Message {
            id: self.id,
            ty: Type::Capability,
            flags: Flags::empty(),
            service: Service::SERVER,
            payload: Payload(to_bytes(&self.capabilities)?),
            ..Default::default()
        })
    }
}

impl TryFrom<Capability> for Message {
    type Error = IntoMessageError;
    fn try_from(c: Capability) -> Result<Self, Self::Error> {
        c.into_message()
    }
}

impl TryFrom<Message> for Capability {
    type Error = IntoCapabilityError;
    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        msg.into_capability()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum IntoCapabilityError {
    #[error("message {0} is not of type \"{}\"", Type::Capability)]
    BadType(Message),

    #[error("map deserialization error: {0}")]
    MapDeserialization(#[from] qi_format::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_header_size() {
        assert_eq!(Header::SIZE, 28);
    }

    fn samples() -> [Message; 3] {
        [
            Message {
                header: Header {
                    id: 123,
                    ty: Type::Post,
                    version: Version::CURRENT,
                    flags: Flags::RETURN_TYPE,
                    subject: Method::try_from_values(
                        Service::Custom(543.into()),
                        Object::Other(32.into()),
                        action::ObjectMethod::Terminate,
                    )
                    .unwrap(),
                },
                payload: vec![1, 2, 3],
            },
            Message {
                header: Header {
                    id: 9034,
                    ty: Type::Event,
                    version: Version::CURRENT,
                    flags: Flags::empty(),
                    subject: Method::try_from_values(
                        Service::Custom(90934.into()),
                        Object::Other(178.into()),
                        action::ObjectMethod::Metaobject,
                    )
                    .unwrap(),
                },
                payload: vec![],
            },
            Message {
                header: Header {
                    id: 21932,
                    version: Version::CURRENT,
                    kind: Type::Capability,
                    flags: Flags::DYNAMIC_PAYLOAD,
                    subject: ServiceDirectory {
                        action: action::ServiceDirectoryMethod::UnregisterService,
                    }
                    .into(),
                },
                payload: vec![100, 200, 255],
            },
        ]
    }

    #[test]
    fn test_decode_message_then_deserialize_annotated_value_from_payload() {
        let input = [
            0x42, 0xde, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            // payload
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let message = MessageCodec
            .decode(&mut bytes::BytesMut::from(input.as_slice()))
            .unwrap()
            .unwrap();
        assert_eq!(
            message,
            Message {
                header: Header {
                    id: 990340,
                    version: Version(0),
                    kind: Type::Error,
                    flags: Flags::empty(),
                    subject: Subject::try_from_values(
                        method::service::Id(47),
                        method::object::Id(1),
                        method::action::Id(178)
                    )
                    .unwrap(),
                },
                payload: bytes::Bytes::from_static(&[
                    0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20,
                    0x72, 0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20,
                    0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64
                ]),
            }
        );
        use qi_format::{from_bytes, Dynamic, Type, Value};
        let value: Dynamic = from_bytes(&message.payload).unwrap();
        assert_eq!(
            value,
            Dynamic::from_type_and_value(Type::String, Value::from("The robot is not localized"))
                .unwrap()
        );
    }

    #[test]
    fn test_message_write() {
        use crate::message::*;
        let msg = Message {
            header: Header {
                id: 329,
                version: Version(12),
                kind: Type::Capability,
                flags: Flags::RETURN_TYPE,
                subject: method::ServiceDirectory {
                    action: method::action::ServiceDirectory::ServiceReady,
                }
                .into(),
            },
            payload: vec![0x17, 0x2b, 0xe6, 0x01, 0x5f],
        };
        let mut buf = bytes::BytesMut::new();
        MessageCodec.encode(msg, &mut buf).unwrap();
        assert_eq!(
            &buf,
            &[
                0x42, 0xde, 0xad, 0x42, // cookie
                0x49, 0x01, 0x00, 0x00, // id
                0x05, 0x00, 0x00, 0x00, // size
                0x0c, 0x00, 0x06, 0x02, // version, type, flags
                0x01, 0x00, 0x00, 0x00, // service
                0x01, 0x00, 0x00, 0x00, // object
                0x68, 0x00, 0x00, 0x00, // action
                0x17, 0x2b, 0xe6, 0x01, 0x5f, // payload
            ][..]
        );
    }

    #[test]
    fn test_message_read_bad_cookie() {
        let input = [
            0x42, 0xdf, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let message = MessageCodec.decode(&mut bytes::BytesMut::from(input.as_slice()));
        assert_matches!(message, Err(DecodeError::BadMagicCookie(0x42dfad42)));
    }

    #[test]
    fn test_message_read_not_enough_data() {
        let input = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x03, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0xb2, 0x00, 0x00, // action, 1 byte short
        ];
        let message = MessageCodec.decode(&mut bytes::BytesMut::from(input.as_slice()));
        assert_matches!(message, Ok(None));
    }
}
