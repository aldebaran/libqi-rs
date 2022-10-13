//! # Message structure
//! ```text
//! ╔═══════════════╤═══════════════╤═══════════════╤═══════════════╗
//! ║       1       │       2       │       3       │       4       ║
//! ╟─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─╢
//! ║0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7║
//! ╠═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╣
//! ║                         magic cookie                          ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                          identifier                           ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                         payload size                          ║
//! ╟───────────────────────────────┬───────────────┬───────────────╢
//! ║            version            │     type      │    flags      ║
//! ╟───────────────────────────────┴───────────────┴───────────────╢
//! ║                            service                            ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                            object                             ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                            action                             ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                            payload                            ║
//! ║                            [ ... ]                            ║
//! ╚═══════════════════════════════════════════════════════════════╝
//! ```
//!  - magic cookie: uint32, big endian, value = 0x42dead42
//!  - id: uint32, little endian
//!  - size/len: uint32, little endian, size of the payload. may be 0
//!  - version: uint16, little endian
//!  - type: uint8, little endian
//!  - flags: uint8, little endian
//!  - service: uint32, little endian
//!  - object: uint32, little endian
//!  - action: uint32, little endian
//!  - payload: 'size' bytes
pub mod kind;
pub use kind::Kind;

pub mod flags;
pub use flags::Flags;

use super::utils::{read_u16_le, read_u32_be, read_u32_le, read_u8};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("bad message magic cookie")]
    BadMagicCookie,
    #[error("unsupported protocol version")]
    UnsupportedVersion,
    #[error("payload size too large")]
    PayloadSizeTooLarge,
    #[error("invalid value")]
    InvalidValue,
    #[error("io error")]
    Io(#[from] IoError),
}

pub type Result<T> = std::result::Result<T, Error>;

const ACTION_ID_CONNECT: u32 = 4;
const ACTION_ID_AUTHENTICATE: u32 = 8;

#[derive(FromPrimitive, ToPrimitive, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u32)]
pub enum ServerAction {
    Connect = ACTION_ID_CONNECT,
    Authenticate = ACTION_ID_AUTHENTICATE,
}

impl Default for ServerAction {
    fn default() -> Self {
        Self::Connect
    }
}

const ACTION_ID_SD_SERVICE: u32 = 100;
const ACTION_ID_SD_SERVICES: u32 = 101;
const ACTION_ID_SD_REGISTER_SERVICE: u32 = 102;
const ACTION_ID_SD_UNREGISTER_SERVICE: u32 = 103;
const ACTION_ID_SD_SERVICE_READY: u32 = 104;
const ACTION_ID_SD_UPDATE_SERVICE_INFO: u32 = 105;
const ACTION_ID_SD_SERVICE_ADDED: u32 = 106;
const ACTION_ID_SD_SERVICE_REMOVED: u32 = 107;
const ACTION_ID_SD_MACHINE_ID: u32 = 108;

#[derive(FromPrimitive, ToPrimitive, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u32)]
pub enum ServiceDirectoryAction {
    Service = ACTION_ID_SD_SERVICE,
    Services = ACTION_ID_SD_SERVICES,
    RegisterService = ACTION_ID_SD_REGISTER_SERVICE,
    UnregisterService = ACTION_ID_SD_UNREGISTER_SERVICE,
    ServiceReady = ACTION_ID_SD_SERVICE_READY,
    UpdateServiceInfo = ACTION_ID_SD_UPDATE_SERVICE_INFO,
    ServiceAdded = ACTION_ID_SD_SERVICE_ADDED,
    ServiceRemoved = ACTION_ID_SD_SERVICE_REMOVED,
    MachineId = ACTION_ID_SD_MACHINE_ID,
}

impl Default for ServiceDirectoryAction {
    fn default() -> Self {
        Self::Service
    }
}

const ACTION_ID_REGISTER_EVENT: u32 = 0;
const ACTION_ID_UNREGISTER_EVENT: u32 = 1;
const ACTION_ID_METAOBJECT: u32 = 2;
const ACTION_ID_TERMINATE: u32 = 3;
const ACTION_ID_PROPERTY: u32 = 5; // not a typo, there is no action 4
const ACTION_ID_SET_PROPERTY: u32 = 6;
const ACTION_ID_PROPERTIES: u32 = 7;
const ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE: u32 = 8;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u32)]
pub enum BoundObjectAction {
    RegisterEvent,
    UnregisterEvent,
    Metaobject,
    Terminate,
    Property,
    SetProperty,
    Properties,
    RegisterEventWithSignature,
    BoundFunction(u32),
}

impl Default for BoundObjectAction {
    fn default() -> Self {
        Self::RegisterEvent
    }
}

impl FromPrimitive for BoundObjectAction {
    fn from_u32(n: u32) -> Option<Self> {
        Some(match n {
            ACTION_ID_REGISTER_EVENT => Self::RegisterEvent,
            ACTION_ID_UNREGISTER_EVENT => Self::UnregisterEvent,
            ACTION_ID_METAOBJECT => Self::Metaobject,
            ACTION_ID_TERMINATE => Self::Terminate,
            ACTION_ID_PROPERTY => Self::Property,
            ACTION_ID_SET_PROPERTY => Self::SetProperty,
            ACTION_ID_PROPERTIES => Self::Properties,
            ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE => Self::RegisterEventWithSignature,
            _ => Self::BoundFunction(n),
        })
    }

    fn from_i64(n: i64) -> Option<Self> {
        Self::from_u32(n.try_into().ok()?)
    }

    fn from_u64(n: u64) -> Option<Self> {
        Self::from_u32(n.try_into().ok()?)
    }
}

impl ToPrimitive for BoundObjectAction {
    fn to_u32(&self) -> Option<u32> {
        Some(match self {
            BoundObjectAction::RegisterEvent => ACTION_ID_REGISTER_EVENT,
            BoundObjectAction::UnregisterEvent => ACTION_ID_UNREGISTER_EVENT,
            BoundObjectAction::Metaobject => ACTION_ID_METAOBJECT,
            BoundObjectAction::Terminate => ACTION_ID_TERMINATE,
            BoundObjectAction::Property => ACTION_ID_PROPERTY,
            BoundObjectAction::SetProperty => ACTION_ID_SET_PROPERTY,
            BoundObjectAction::Properties => ACTION_ID_PROPERTIES,
            BoundObjectAction::RegisterEventWithSignature => {
                ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE
            }
            BoundObjectAction::BoundFunction(n) => *n,
        })
    }

    fn to_i64(&self) -> Option<i64> {
        Some(self.to_u32().unwrap().into())
    }

    fn to_u64(&self) -> Option<u64> {
        Some(self.to_u32().unwrap().into())
    }
}

const SERVICE_ID_SERVER: u32 = 0;
const SERVICE_ID_SERVICE_DIRECTORY: u32 = 1;

const OBJECT_ID_NONE: u32 = 0;
const OBJECT_ID_SERVICE_MAIN: u32 = 1;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Target {
    // service = server, object = none
    Server(ServerAction),
    // service = sd, object = service main
    ServiceDirectory(ServiceDirectoryAction),
    // other
    BoundObject {
        service: u32,
        object: u32,
        action: BoundObjectAction,
    },
}

impl Target {
    fn from_values(service: u32, object: u32, action: u32) -> Option<Self> {
        match (service, object, action) {
            (SERVICE_ID_SERVER, OBJECT_ID_NONE, action) => {
                Some(Self::Server(ServerAction::from_u32(action)?))
            }
            (SERVICE_ID_SERVICE_DIRECTORY, OBJECT_ID_SERVICE_MAIN, action) => Some(
                Self::ServiceDirectory(ServiceDirectoryAction::from_u32(action)?),
            ),
            (service, object, action)
                if service != SERVICE_ID_SERVER && object != OBJECT_ID_NONE =>
            {
                Some(Self::BoundObject {
                    service,
                    object,
                    action: BoundObjectAction::from_u32(action).unwrap(),
                })
            }
            _ => None,
        }
    }

    fn service(&self) -> u32 {
        match self {
            Self::Server(_) => SERVICE_ID_SERVER,
            Self::ServiceDirectory(_) => SERVICE_ID_SERVICE_DIRECTORY,
            Self::BoundObject { service, .. } => *service,
        }
    }

    fn object(&self) -> u32 {
        match self {
            Self::Server(_) => OBJECT_ID_NONE,
            Self::ServiceDirectory(_) => OBJECT_ID_SERVICE_MAIN,
            Self::BoundObject { object, .. } => *object,
        }
    }

    fn action(&self) -> u32 {
        match self {
            Self::Server(act) => act.to_u32(),
            Self::ServiceDirectory(act) => act.to_u32(),
            Self::BoundObject { action, .. } => action.to_u32(),
        }
        .unwrap()
    }

    async fn write<W>(&self, mut writer: W) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        writer.write_all(&self.service().to_le_bytes()).await?;
        writer.write_all(&self.object().to_le_bytes()).await?;
        writer.write_all(&self.action().to_le_bytes()).await?;
        Ok(())
    }

    async fn read<R>(mut reader: R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let service = read_u32_le(&mut reader).await?;
        let object = read_u32_le(&mut reader).await?;
        let action = read_u32_le(&mut reader).await?;

        Self::from_values(service, object, action).ok_or(Error::InvalidValue)
    }
}

impl Default for Target {
    fn default() -> Self {
        Self::Server(ServerAction::default())
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Message {
    pub id: u32,
    pub kind: Kind, // or type
    pub flags: Flags,
    pub target: Target,
    pub payload: Vec<u8>,
}

impl Message {
    const VERSION: u16 = 0;
    const MAGIC_COOKIE: u32 = 0x42dead42;

    pub fn new() -> Self {
        Self {
            id: 0,
            kind: Kind::None,
            flags: Flags::empty(),
            target: Target::default(),
            payload: Vec::new(),
        }
    }

    pub async fn write<W>(&self, mut writer: W) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        unimplemented!()
        //let payload_size = self.payload.len();
        //let payload_size: u32 = match payload_size.try_into() {
        //    Ok(size) => size,
        //    Err(err) => {
        //        return Err(
        //            IoError::new(IoErrorKind::Other, format!("bad payload size: {err}")).into(),
        //        )
        //    }
        //};

        //writer.write_all(&Self::MAGIC_COOKIE.to_be_bytes()).await?;
        //writer.write_all(&self.id.to_le_bytes()).await?;
        //writer.write_all(&payload_size.to_le_bytes()).await?;
        //writer.write_all(&Self::VERSION.to_le_bytes()).await?;
        //self.kind.write(&mut writer).await?;
        //self.flags.write(&mut writer).await?;
        //self.target.write(&mut writer).await?;
        //writer.write_all(&self.payload).await?;
        //Ok(())
    }

    pub async fn read<R>(mut reader: R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        unimplemented!()
        //let magic_cookie = read_u32_be(&mut reader).await?;
        //if magic_cookie != Self::MAGIC_COOKIE {
        //    return Err(Error::BadMagicCookie);
        //}

        //let id = read_u32_le(&mut reader).await?;
        //let payload_size = read_u32_le(&mut reader).await?;
        //let payload_size = payload_size
        //    .try_into()
        //    .map_err(|_| Error::PayloadSizeTooLarge)?;
        //let version = read_u16_le(&mut reader).await?;
        //if version != Self::VERSION {
        //    return Err(Error::UnsupportedVersion);
        //}
        //let kind = Kind::read(&mut reader).await?;
        //let flags = Flags::read(&mut reader).await?;
        //let target = Target::read(&mut reader).await?;
        //let mut payload = vec![0; payload_size];
        //reader.read_exact(&mut payload).await?;

        //Ok(Self {
        //    id,
        //    kind,
        //    flags,
        //    target,
        //    payload,
        //})
    }
}

impl Default for Message {
    fn default() -> Self {
        Message::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use futures_test::test;
    use pretty_assertions::assert_eq;

    #[test]
    async fn message_write() {
        let msg = Message {
            id: 329,
            kind: Kind::Capability,
            flags: Flags::RETURN_TYPE,
            target: Target::ServiceDirectory(ServiceDirectoryAction::ServiceReady),
            payload: vec![23u8, 43u8, 230u8, 1u8, 95u8],
        };
        let mut buf = Vec::new();
        msg.write(&mut buf).await.expect("write error");
        let expected = vec![
            0x42, 0xde, 0xad, 0x42, // cookie
            0x49, 0x01, 0x00, 0x00, // id
            0x05, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x06, 0x02, // version, type, flags
            0x01, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0x68, 0x00, 0x00, 0x00, // action
            0x17, 0x2b, 0xe6, 0x01, 0x5f, // payload
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    async fn message_read() {
        let input = &[
            0x42, 0xde, 0xad, 0x42, // cookie
            0xb8, 0x9a, 0x00, 0x00, // id
            0x28, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x02, 0x00, // version, type, flags
            0x27, 0x00, 0x00, 0x00, // service
            0x09, 0x00, 0x00, 0x00, // object
            0x68, 0x00, 0x00, 0x00, // action
            // payload
            0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d, 0x65,
            0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35, 0x32, 0x2d,
            0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30, 0x33,
            // garbage at the end, should be ignored
            0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
            0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
        ];
        let msg = Message::read(input.as_slice()).await.expect("read error");
        let expected = Message {
            id: 39608,
            kind: Kind::Reply,
            flags: Flags::empty(),
            target: Target::BoundObject {
                service: 39,
                object: 9,
                action: BoundObjectAction::BoundFunction(104),
            },
            payload: vec![
                0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d, 0x65,
                0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35, 0x32, 0x2d,
                0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30, 0x33,
            ],
        };
        assert_eq!(msg, expected);
    }

    #[test]
    async fn message_read_bad_cookie() {
        let input = &[
            0x42, 0xde, 0xad, 0x00, // bad cookie
            0xb8, 0x9a, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x27, 0x00,
            0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x68, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00,
            0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d, 0x65, 0x30, 0x37, 0x66, 0x2d,
            0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35, 0x32, 0x2d, 0x39, 0x39, 0x35, 0x37,
            0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30,
            0x33, // garbage at the end, should be ignored
            0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
            0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
        ];
        let res = Message::read(input.as_slice()).await;
        assert_matches!(res, Err(Error::BadMagicCookie));
    }

    #[test]
    async fn message_write_read_invariant() {
        let msg = Message {
            id: 9323982,
            kind: Kind::Error,
            flags: Flags::DYNAMIC_PAYLOAD | Flags::RETURN_TYPE,
            target: Target::BoundObject {
                service: 984398294,
                object: 87438426,
                action: BoundObjectAction::SetProperty,
            },
            payload: vec![0x10, 0x11, 0x12, 0x13, 0x15],
        };
        let mut buffer = Vec::new();
        msg.write(&mut buffer).await.expect("write error");
        let msg2 = Message::read(buffer.as_slice()).await.expect("read error");
        assert_eq!(msg, msg2);
    }
}
