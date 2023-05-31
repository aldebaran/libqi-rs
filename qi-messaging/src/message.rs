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
//! ║ ├ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - ┤ ║
//! ║ │                            object                             │ ║
//! ║ ├ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - ┤ ║
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
//!  - subject, 3 x 4 bytes unsigned integer, all little endian
//!    - service
//!    - object
//!    - action
//!
//!  The total header size is therefore 28 bytes.

use crate::{capabilities, format, types::Dynamic};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use tracing::{instrument, warn};

macro_rules! impl_u32_le_field {
    ($($name:ident),+) => {
        $(
            impl $name {
                const SIZE: usize = std::mem::size_of::<u32>();

                pub const fn new(value: u32) -> Self {
                    Self(value)
                }

                fn read<B>(buf: &mut B) -> Self
                where
                    B: Buf,
                {
                    Self(buf.get_u32_le())
                }

                fn write<B>(self, buf: &mut B)
                where
                    B: BufMut,
                {
                    buf.put_u32_le(self.0)
                }
            }
        )+
    }
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
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub(crate) struct Id(pub u32);

#[derive(
    Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, derive_more::Display,
)]
struct Version(u16);

impl Version {
    const SIZE: usize = std::mem::size_of::<u16>();
    const CURRENT: Self = Self(0);

    fn read<B>(buf: &mut B) -> Self
    where
        B: Buf,
    {
        Self(buf.get_u16_le())
    }

    fn write<B>(self, buf: &mut B)
    where
        B: BufMut,
    {
        buf.put_u16_le(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display)]
pub enum Subject {
    Control(ControlSubject),
    Service(ServiceSubject),
}

impl Default for Subject {
    fn default() -> Self {
        Self::Control(ControlSubject::default())
    }
}

impl Subject {
    const SIZE: usize = 3 * std::mem::size_of::<u32>();

    pub const fn new(service: Service, object: Object, action: Action) -> Self {
        match (service, object) {
            (ControlSubject::SERVICE, ControlSubject::OBJECT) => {
                Self::Control(ControlSubject(action))
            }
            _ => Self::Service(ServiceSubject(service, object, action)),
        }
    }

    pub const fn control(action: Action) -> Self {
        Self::Control(ControlSubject(action))
    }

    fn read<B>(buf: &mut B) -> Self
    where
        B: Buf,
    {
        let service = Service::read(buf);
        let object = Object::read(buf);
        let action = Action::read(buf);
        Self::new(service, object, action)
    }

    fn write<B>(self, buf: &mut B)
    where
        B: BufMut,
    {
        self.service().write(buf);
        self.object().write(buf);
        self.action().write(buf);
    }

    pub const fn service(&self) -> Service {
        match self {
            Subject::Control(s) => s.service(),
            Subject::Service(s) => s.service(),
        }
    }

    pub const fn object(&self) -> Object {
        match self {
            Subject::Control(s) => s.object(),
            Subject::Service(s) => s.object(),
        }
    }

    pub const fn action(&self) -> Action {
        match self {
            Subject::Control(s) => s.action(),
            Subject::Service(s) => s.action(),
        }
    }
}

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
#[display(fmt = "control({_0})")]
pub struct ControlSubject(Action);

impl ControlSubject {
    const SERVICE: Service = Service(0);
    const OBJECT: Object = Object(0);

    pub const fn service(&self) -> Service {
        Self::SERVICE
    }

    pub const fn object(&self) -> Object {
        Self::OBJECT
    }

    pub const fn action(&self) -> Action {
        self.0
    }
}

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
#[display(fmt = "({_0}, {_1}, {_2})")]
pub struct ServiceSubject(Service, Object, Action);

impl ServiceSubject {
    pub const fn service(&self) -> Service {
        self.0
    }

    pub const fn object(&self) -> Object {
        self.1
    }

    pub const fn action(&self) -> Action {
        self.2
    }
}

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
pub struct Service(u32);

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
pub struct Object(u32);

#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::Display,
)]
pub struct Action(u32);

impl Action {}

impl_u32_le_field!(Id, Service, Object, Action);

#[derive(
    Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, derive_more::UpperHex,
)]
#[upper_hex(fmt = "{:#X}", "Self::VALUE")]
struct MagicCookie;

impl MagicCookie {
    const SIZE: usize = std::mem::size_of::<u32>();
    const VALUE: u32 = 0x42dead42;

    fn read<B>(buf: &mut B) -> Result<Self, InvalidMagicCookieValueError>
    where
        B: Buf,
    {
        let value = buf.get_u32();
        if value == Self::VALUE {
            Ok(MagicCookie)
        } else {
            Err(InvalidMagicCookieValueError(value))
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
#[error("invalid message magic cookie value {0:x}")]
pub(crate) struct InvalidMagicCookieValueError(u32);

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct PayloadSize(usize);

impl PayloadSize {
    const SIZE: usize = std::mem::size_of::<u32>();

    fn read<B>(buf: &mut B) -> Result<Self, PayloadCannotBeRepresentedAsUSizeError>
    where
        B: Buf,
    {
        let size = buf.get_u32_le();
        if size > (usize::MAX as u32) {
            return Err(PayloadCannotBeRepresentedAsUSizeError(size));
        }
        let size = size as usize;
        Ok(Self(size))
    }

    fn write<B>(self, buf: &mut B) -> Result<(), PayloadCannotBeRepresentedAsU32Error>
    where
        B: BufMut,
    {
        let size = self.0;
        if size > (u32::MAX as usize) {
            return Err(PayloadCannotBeRepresentedAsU32Error(size));
        }
        let size = size as u32;
        buf.put_u32_le(size);
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
#[error(
    "message payload size {0} cannot be represented as an usize (the maximum for this system is {})",
    usize::MAX
)]
pub(crate) struct PayloadCannotBeRepresentedAsUSizeError(u32);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
#[error(
    "message payload size {0} cannot be represented as an u32 (the maximum for this system is {})",
    u32::MAX
)]
pub(crate) struct PayloadCannotBeRepresentedAsU32Error(usize);

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
pub(crate) enum Kind {
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

impl Kind {
    const SIZE: usize = std::mem::size_of::<u8>();

    fn read<B>(buf: &mut B) -> Result<Self, InvalidKindValueError>
    where
        B: Buf,
    {
        buf.get_u8().try_into()
    }

    fn write<B>(self, buf: &mut B)
    where
        B: BufMut,
    {
        buf.put_u8(self.into())
    }
}

impl Default for Kind {
    fn default() -> Self {
        Self::Call
    }
}

impl From<Kind> for u8 {
    fn from(kind: Kind) -> u8 {
        use num_traits::ToPrimitive;
        kind.to_u8().unwrap()
    }
}

impl std::convert::TryFrom<u8> for Kind {
    type Error = InvalidKindValueError;

    fn try_from(value: u8) -> Result<Self, InvalidKindValueError> {
        use num_traits::FromPrimitive;
        Self::from_u8(value).ok_or(InvalidKindValueError(value))
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, thiserror::Error)]
#[error("invalid message kind value {0}")]
pub(crate) struct InvalidKindValueError(u8);

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

    fn set_dynamic_payload(&mut self, value: bool) {
        self.set(Self::DYNAMIC_PAYLOAD, value);
    }

    fn set_return_type(&mut self, value: bool) {
        self.set(Self::RETURN_TYPE, value);
    }

    fn read<B>(buf: &mut B) -> Result<Self, InvalidFlagsValueError>
    where
        B: Buf,
    {
        let byte = buf.get_u8();
        let flags = Self::try_from(byte)?;
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
pub struct InvalidFlagsValueError(u8);

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct Header {
    id: Id,
    kind: Kind,
    payload_size: usize,
    flags: Flags,
    subject: Subject,
}

impl Header {
    const MAGIC_COOKIE_OFFSET: usize = 0;
    const ID_OFFSET: usize = Self::MAGIC_COOKIE_OFFSET + MagicCookie::SIZE;
    const PAYLOAD_SIZE_OFFSET: usize = Self::ID_OFFSET + Id::SIZE;
    const VERSION_OFFSET: usize = Self::PAYLOAD_SIZE_OFFSET + PayloadSize::SIZE;
    const TYPE_OFFSET: usize = Self::VERSION_OFFSET + Version::SIZE;
    const FLAGS_OFFSET: usize = Self::TYPE_OFFSET + Kind::SIZE;
    const SUBJECT_OFFSET: usize = Self::FLAGS_OFFSET + Flags::SIZE;
    const SIZE: usize = Self::SUBJECT_OFFSET + Subject::SIZE;

    fn read<B>(buf: &mut B) -> Result<Self, ReadHeaderError>
    where
        B: Buf,
    {
        MagicCookie::read(buf)?;
        let id = Id::read(buf);
        let payload_size = PayloadSize::read(buf)?.0;
        let version = Version::read(buf);
        if version != Version::CURRENT {
            return Err(ReadHeaderError::UnsupportedVersion(version.0));
        }
        let ty = Kind::read(buf)?;
        let flags = Flags::read(buf)?;
        let subject = Subject::read(buf);
        Ok(Self {
            id,
            kind: ty,
            payload_size,
            flags,
            subject,
        })
    }

    fn write<B>(self, buf: &mut B) -> Result<(), WriteHeaderError>
    where
        B: BufMut,
    {
        let mut hbuf = [0u8; Header::SIZE];
        let mut hbuf_ref = hbuf.as_mut();
        MagicCookie.write(&mut hbuf_ref);
        self.id.write(&mut hbuf_ref);
        PayloadSize(self.payload_size).write(&mut hbuf_ref)?;
        Version::CURRENT.write(&mut hbuf_ref);
        self.kind.write(&mut hbuf_ref);
        self.flags.write(&mut hbuf_ref);
        self.subject.write(&mut hbuf_ref);
        buf.put(hbuf.as_ref());
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub(crate) enum ReadHeaderError {
    #[error(transparent)]
    MagicCookie(#[from] InvalidMagicCookieValueError),

    #[error(transparent)]
    PayloadSize(#[from] PayloadCannotBeRepresentedAsUSizeError),

    #[error("unsupported message version {0}")]
    UnsupportedVersion(u16),

    #[error(transparent)]
    Kind(#[from] InvalidKindValueError),

    #[error(transparent)]
    Flags(#[from] InvalidFlagsValueError),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub(crate) enum WriteHeaderError {
    #[error(transparent)]
    PayloadSize(#[from] PayloadCannotBeRepresentedAsU32Error),
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, derive_more::Display)]
#[display(fmt = "message(id={id}, {kind}, subject={subject}, flags={flags})")]
pub(crate) struct Message {
    id: Id,
    kind: Kind,
    subject: Subject,
    flags: Flags,
    payload: Bytes,
}

impl Message {
    fn new(header: Header, payload: Bytes) -> Self {
        Self {
            id: header.id,
            kind: header.kind,
            subject: header.subject,
            flags: header.flags,
            payload,
        }
    }

    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Builds a "call" message.
    ///
    /// This sets the kind, the id and the subject of the message.
    pub fn call(id: Id, subject: Subject) -> Builder {
        Builder::new()
            .set_id(id)
            .set_kind(Kind::Call)
            .set_subject(subject)
    }

    /// Builds a "reply" message.
    ///
    /// This sets the kind, the id and the subject of the message.
    pub fn reply(id: Id, subject: Subject) -> Builder {
        Builder::new()
            .set_id(id)
            .set_kind(Kind::Reply)
            .set_subject(subject)
    }

    /// Builds a "error" message.
    ///
    /// This sets the kind, the id, the subject and the payload of the message.
    pub fn error(id: Id, subject: Subject, description: &str) -> Result<Builder, format::Error> {
        Builder::new()
            .set_id(id)
            .set_kind(Kind::Error)
            .set_subject(subject)
            .set_error_description(description)
    }

    /// Builds a "post" message.
    ///
    /// This sets the kind, the id and the subject of the message.
    pub fn post(id: Id, subject: Subject) -> Builder {
        Builder::new()
            .set_id(id)
            .set_kind(Kind::Post)
            .set_subject(subject)
    }

    /// Builds a "event" message.
    ///
    /// This sets the kind, the id and the subject of the message.
    pub fn event(id: Id, subject: Subject) -> Builder {
        Builder::new()
            .set_id(id)
            .set_kind(Kind::Event)
            .set_subject(subject)
    }

    /// Builds a "capabilities" message.
    ///
    /// This sets the kind, the id, the subject and the payload of the message.
    pub fn capabilities(id: Id, map: &capabilities::Map) -> Result<Builder, format::Error> {
        Builder::new()
            .set_id(id)
            .set_kind(Kind::Capabilities)
            .set_control_subject(None)
            .set_value(&map)
    }

    /// Builds a "cancel" message.
    ///
    /// This sets the kind, the id, the subject and the payload of the message.
    pub fn cancel(id: Id, subject: Subject, call_id: Id) -> Builder {
        Builder::new()
            .set_id(id)
            .set_kind(Kind::Cancel)
            .set_subject(subject)
            .set_value(&call_id)
            .expect("failed to serialize a message ID in the format")
    }

    /// Builds a "canceled" message.
    ///
    /// This sets the kind, the id and the subject of the message.
    pub fn canceled(id: Id, subject: Subject) -> Builder {
        Builder::new()
            .set_id(id)
            .set_subject(subject)
            .set_kind(Kind::Canceled)
    }

    fn write<B>(self, buf: &mut B) -> Result<(), WriteHeaderError>
    where
        B: BufMut,
    {
        Header {
            id: self.id,
            kind: self.kind,
            payload_size: self.payload.len(),
            flags: self.flags,
            subject: self.subject,
        }
        .write(buf)?;
        buf.put(self.payload);
        Ok(())
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn subject(&self) -> Subject {
        self.subject
    }

    pub fn payload(&self) -> Bytes {
        self.payload.clone()
    }

    pub fn into_payload(self) -> Bytes {
        self.payload
    }

    pub fn size(&self) -> usize {
        Header::SIZE + self.payload.len()
    }

    pub fn value<T>(&self) -> Result<T, format::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        format::from_bytes(&self.payload)
    }

    pub fn error_description(&self) -> Result<String, GetErrorDescriptionError> {
        let dynamic: Dynamic = self.value()?;
        match dynamic {
            Dynamic::String(s) => Ok(s),
            d => Err(GetErrorDescriptionError::DynamicValueIsNotAString(d)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetErrorDescriptionError {
    #[error("dynamic value {0} of error description is not a string")]
    DynamicValueIsNotAString(Dynamic),

    #[error(transparent)]
    Format(#[from] format::Error),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct Builder(Message);

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Self(Message::default())
    }

    pub fn set_id(mut self, value: Id) -> Self {
        self.0.id = value;
        self
    }

    pub(crate) fn set_kind(mut self, value: Kind) -> Self {
        self.0.kind = value;
        self
    }

    pub(crate) fn set_flags(mut self, value: Flags) -> Self {
        self.0.flags = value;
        self
    }

    pub fn set_subject(mut self, value: Subject) -> Self {
        self.0.subject = value;
        self
    }

    pub fn set_control_subject(self, action: Option<Action>) -> Self {
        self.set_subject(Subject::control(action.unwrap_or_default()))
    }

    pub fn set_payload(mut self, value: Bytes) -> Self {
        self.0.payload = value;
        self
    }

    /// Sets the serialized representation of the value in the format as the payload of the message.
    /// It checks if the "dynamic payload" flag is set on the message to know how to serialize the value.
    /// If the flag is set after calling this value, the value will not be serialized coherently with the flag.
    pub fn set_value<T>(mut self, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        if self.0.flags.contains(Flags::DYNAMIC_PAYLOAD) {
            todo!("serialize a value as a dynamic")
        } else {
            self.0.payload = format::to_bytes(value)?;
        };
        Ok(self)
    }

    pub fn set_error_description(self, description: &str) -> Result<Self, format::Error> {
        self.set_value(&Dynamic::from(description))
    }

    pub fn build(self) -> Message {
        self.0
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub(crate) struct Encoder;

impl tokio_util::codec::Encoder<Message> for Encoder {
    type Error = EncodeError;

    #[instrument(name = "encode", skip_all, err)]
    fn encode(&mut self, msg: Message, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        dst.reserve(msg.size());
        msg.write(dst)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EncodeError {
    #[error("write header error")]
    WriteHeader(#[from] WriteHeaderError),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub(crate) struct Decoder {
    state: DecoderState,
}

impl Decoder {
    pub(crate) fn new() -> Self {
        Self {
            state: DecoderState::Header,
        }
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl tokio_util::codec::Decoder for Decoder {
    type Item = Message;
    type Error = DecodeError;

    #[instrument(name = "decode", skip_all, err)]
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let msg = loop {
            match self.state {
                DecoderState::Header => match decode_header(src)? {
                    None => break None,
                    Some(header) => self.state = DecoderState::Payload(header),
                },
                DecoderState::Payload(header) => match decode_payload(header.payload_size, src) {
                    None => break None,
                    Some(payload) => {
                        self.state = DecoderState::Header;
                        src.reserve(src.len());
                        break Some(Message::new(header, payload));
                    }
                },
            }
        };
        Ok(msg)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DecodeError {
    #[error("read header error")]
    ReadHeader(#[from] ReadHeaderError),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
enum DecoderState {
    Header,
    Payload(Header),
}

#[instrument(skip_all)]
fn decode_header(src: &mut bytes::BytesMut) -> Result<Option<Header>, DecodeError> {
    if src.len() < Header::SIZE {
        src.reserve(Header::SIZE - src.len());
        return Ok(None);
    }

    let header = Header::read(&mut src.as_ref())?;
    src.advance(Header::SIZE);
    Ok(Some(header))
}

#[instrument(skip_all)]
fn decode_payload(size: usize, src: &mut BytesMut) -> Option<Bytes> {
    if src.len() < size {
        src.reserve(size - src.len());
        return None;
    }
    Some(src.copy_to_bytes(size))
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
    fn test_header_read() {
        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
        ];
        assert_eq!(
            Header::read(&mut input),
            Ok(Header {
                id: Id(990340),
                kind: Kind::Error,
                payload_size: 35,
                subject: Subject::Service(ServiceSubject(Service(47), Object(1), Action(178))),
                flags: Flags::empty(),
            })
        );
    }

    #[test]
    fn test_message_write() {
        use crate::message::*;
        let msg = Message {
            id: Id(329),
            kind: Kind::Capabilities,
            subject: Subject::Service(ServiceSubject(Service(1), Object(1), Action(104))),
            flags: Flags::RETURN_TYPE,
            payload: Bytes::from_static(&[0x17, 0x2b, 0xe6, 0x01, 0x5f]),
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
                0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, 0x00, 0x00,
                0x00, // subject,
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
            Err(ReadHeaderError::MagicCookie(InvalidMagicCookieValueError(
                0x42dfad42
            )))
        );
    }

    #[test]
    fn test_header_read_invalid_type_value() {
        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0xaa, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00, // subject
        ];
        let header = Header::read(&mut input);
        assert_eq!(
            header,
            Err(ReadHeaderError::Kind(InvalidKindValueError(0xaa)))
        );
    }

    #[test]
    fn test_header_read_invalid_flags_value() {
        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x03, 0x13, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00, // subject
        ];
        let header = Header::read(&mut input);
        assert_eq!(
            header,
            Err(ReadHeaderError::Flags(InvalidFlagsValueError(0x13)))
        );
    }

    #[test]
    fn test_header_read_unsupported_version() {
        let mut input: &[u8] = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x12, 0x34, 0x03, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00, // subject
        ];
        let header = Header::read(&mut input);
        assert_eq!(header, Err(ReadHeaderError::UnsupportedVersion(0x3412)));
    }

    #[test]
    fn test_decoder_not_enough_data_for_header() {
        todo!()
    }

    #[test]
    fn test_decoder_not_enough_data_for_payload() {
        todo!()
    }

    #[test]
    fn test_decoder_garbage() {
        todo!()
    }

    #[test]
    fn test_decoder_success() {
        todo!()
    }

    #[test]
    fn test_encoder_bad_payload_size() {
        todo!()
    }

    #[test]
    fn test_encoder_success() {
        todo!()
    }
}
