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
pub use action::Action;

pub mod subject;
pub use subject::Subject;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("bad message magic cookie")]
    BadMagicCookie,
    #[error("unsupported protocol version")]
    UnsupportedVersion,
    #[error("payload size too large")]
    PayloadSizeTooLarge,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Message {
    pub id: u32,
    pub kind: Kind, // or type
    pub flags: Flags,
    pub subject: Subject,
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
            subject: Subject::default(),
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
        //self.subject.write(&mut writer).await?;
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
        //let subject = Subject::read(&mut reader).await?;
        //let mut payload = vec![0; payload_size];
        //reader.read_exact(&mut payload).await?;

        //Ok(Self {
        //    id,
        //    kind,
        //    flags,
        //    subject,
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
pub mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use futures_test::test;
    use pretty_assertions::assert_eq;

    pub fn samples() -> [Message; 3] {
        [
            Message {
                id: 123,
                kind: Kind::Post,
                flags: Flags::RETURN_TYPE,
                subject: subject::BoundObject::from_values_unchecked(
                    subject::Service::Other(543.into()),
                    subject::Object::Other(32.into()),
                    action::BoundObject::Terminate,
                )
                .into(),
                payload: vec![1, 2, 3],
            },
            Message {
                id: 9034,
                kind: Kind::Event,
                flags: Flags::empty(),
                subject: subject::BoundObject::from_values_unchecked(
                    subject::Service::Other(90934.into()),
                    subject::Object::Other(178.into()),
                    action::BoundObject::Metaobject,
                )
                .into(),
                payload: vec![],
            },
            Message {
                id: 21932,
                kind: Kind::Capability,
                flags: Flags::DYNAMIC_PAYLOAD,
                subject: subject::ServiceDirectory {
                    action: action::ServiceDirectory::UnregisterService,
                }
                .into(),
                payload: vec![100, 200, 255],
            },
        ]
    }

    #[test]
    async fn message_write() {
        todo!()
        //let msg = Message {
        //    id: 329,
        //    kind: Kind::Capability,
        //    flags: Flags::RETURN_TYPE,
        //    subject: Subject::ServiceDirectory(ServiceDirectoryAction::ServiceReady),
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
        //    subject: Subject::BoundObject {
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
        //     subject: Subject::BoundObject {
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
