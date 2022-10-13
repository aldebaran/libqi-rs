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
//! ║                            action                             ║
//! ╚═══════════════════════════════════════════════════════════════╝
//! ```
//!
//!  - magic cookie: uint32, big endian, value = 0x42dead42
//!  - id: uint32, little endian
//!  - size/len: uint32, little endian, size of the payload. may be 0
//!  - version: uint16, little endian
//!  - type: uint8, little endian
//!  - flags: uint8, little endian
//!  - service: uint32, little endian
//!  - object: uint32, little endian
//!  - action: uint32, little endian
pub mod kind;
pub use kind::Kind;

pub mod flags;
pub use flags::Flags;

pub mod action;

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
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

const SERVICE_ID_SERVER: u32 = 0;
const SERVICE_ID_SERVICE_DIRECTORY: u32 = 1;

const OBJECT_ID_NONE: u32 = 0;
const OBJECT_ID_SERVICE_MAIN: u32 = 1;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Target {
    // service = server, object = none
    Server(action::Server),
    // service = sd, object = service main
    ServiceDirectory(action::ServiceDirectory),
    // other
    BoundObject {
        service: u32,
        object: u32,
        action: action::BoundObject,
    },
}

impl Target {
    fn from_values(service: u32, object: u32, action: u32) -> Option<Self> {
        use num_traits::FromPrimitive;
        match (service, object, action) {
            (SERVICE_ID_SERVER, OBJECT_ID_NONE, action) => {
                Some(Self::Server(action::Server::from_u32(action)?))
            }
            (SERVICE_ID_SERVICE_DIRECTORY, OBJECT_ID_SERVICE_MAIN, action) => Some(
                Self::ServiceDirectory(action::ServiceDirectory::from_u32(action)?),
            ),
            (service, object, action)
                if service != SERVICE_ID_SERVER && object != OBJECT_ID_NONE =>
            {
                Some(Self::BoundObject {
                    service,
                    object,
                    action: action::BoundObject::from_u32(action).unwrap(),
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
        use num_traits::ToPrimitive;
        match self {
            Self::Server(act) => act.to_u32(),
            Self::ServiceDirectory(act) => act.to_u32(),
            Self::BoundObject { action, .. } => action.to_u32(),
        }
        .unwrap()
    }

    //async fn write<W>(&self, mut writer: W) -> Result<()>
    //where
    //    W: AsyncWrite + Unpin,
    //{
    //    writer.write_all(&self.service().to_le_bytes()).await?;
    //    writer.write_all(&self.object().to_le_bytes()).await?;
    //    writer.write_all(&self.action().to_le_bytes()).await?;
    //    Ok(())
    //}

    //async fn read<R>(mut reader: R) -> Result<Self>
    //where
    //    R: AsyncRead + Unpin,
    //{
    //    let service = read_u32_le(&mut reader).await?;
    //    let object = read_u32_le(&mut reader).await?;
    //    let action = read_u32_le(&mut reader).await?;

    //    Self::from_values(service, object, action).ok_or(Error::InvalidValue)
    //}
}

impl Default for Target {
    fn default() -> Self {
        Self::Server(action::Server::default())
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
//where
    //    W: AsyncWrite + Unpin,
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
//where
    //    R: AsyncRead + Unpin,
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
        todo!()
        //let msg = Message {
        //    id: 329,
        //    kind: Kind::Capability,
        //    flags: Flags::RETURN_TYPE,
        //    target: Target::ServiceDirectory(ServiceDirectoryAction::ServiceReady),
        //    payload: vec![23u8, 43u8, 230u8, 1u8, 95u8],
        //};
        //let mut buf = Vec::new();
        //msg.write(&mut buf).await.expect("write error");
        //let expected = vec![
        //    0x42, 0xde, 0xad, 0x42, // cookie
        //    0x49, 0x01, 0x00, 0x00, // id
        //    0x05, 0x00, 0x00, 0x00, // size
        //    0x00, 0x00, 0x06, 0x02, // version, type, flags
        //    0x01, 0x00, 0x00, 0x00, // service
        //    0x01, 0x00, 0x00, 0x00, // object
        //    0x68, 0x00, 0x00, 0x00, // action
        //    0x17, 0x2b, 0xe6, 0x01, 0x5f, // payload
        //];
        //assert_eq!(buf, expected);
    }

    #[test]
    async fn message_read() {
        todo!()
        //let input = &[
        //    0x42, 0xde, 0xad, 0x42, // cookie
        //    0xb8, 0x9a, 0x00, 0x00, // id
        //    0x28, 0x00, 0x00, 0x00, // size
        //    0x00, 0x00, 0x02, 0x00, // version, type, flags
        //    0x27, 0x00, 0x00, 0x00, // service
        //    0x09, 0x00, 0x00, 0x00, // object
        //    0x68, 0x00, 0x00, 0x00, // action
        //    // payload
        //    0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d, 0x65,
        //    0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35, 0x32, 0x2d,
        //    0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30, 0x33,
        //    // garbage at the end, should be ignored
        //    0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
        //    0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
        //];
        //let msg = Message::read(input.as_slice()).await.expect("read error");
        //let expected = Message {
        //    id: 39608,
        //    kind: Kind::Reply,
        //    flags: Flags::empty(),
        //    target: Target::BoundObject {
        //        service: 39,
        //        object: 9,
        //        action: BoundObjectAction::BoundFunction(104),
        //    },
        //    payload: vec![
        //        0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d, 0x65,
        //        0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35, 0x32, 0x2d,
        //        0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30, 0x33,
        //    ],
        //};
        //assert_eq!(msg, expected);
    }

    #[test]
    async fn message_read_bad_cookie() {
        todo!()
        //let input = &[
        //    0x42, 0xde, 0xad, 0x00, // bad cookie
        //    0xb8, 0x9a, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x27, 0x00,
        //    0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x68, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00,
        //    0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d, 0x65, 0x30, 0x37, 0x66, 0x2d,
        //    0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35, 0x32, 0x2d, 0x39, 0x39, 0x35, 0x37,
        //    0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30,
        //    0x33, // garbage at the end, should be ignored
        //    0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
        //    0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
        //];
        //let res = Message::read(input.as_slice()).await;
        //assert_matches!(res, Err(Error::BadMagicCookie));
    }

    #[test]
    async fn message_write_read_invariant() {
        todo!()
        // let msg = Message {
        //     id: 9323982,
        //     kind: Kind::Error,
        //     flags: Flags::DYNAMIC_PAYLOAD | Flags::RETURN_TYPE,
        //     target: Target::BoundObject {
        //         service: 984398294,
        //         object: 87438426,
        //         action: BoundObjectAction::SetProperty,
        //     },
        //     payload: vec![0x10, 0x11, 0x12, 0x13, 0x15],
        // };
        // let mut buffer = Vec::new();
        // msg.write(&mut buffer).await.expect("write error");
        // let msg2 = Message::read(buffer.as_slice()).await.expect("read error");
        // assert_eq!(msg, msg2);
    }
}
