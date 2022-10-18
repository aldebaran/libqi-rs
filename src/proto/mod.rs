mod de;
pub use de::{from_bytes, from_reader, Deserializer};
pub mod message;
pub use message::Message;
mod ser;
pub use ser::{to_bytes, to_writer, Serializer};
pub mod utils;
pub mod value;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("size {0} conversion failed: {1}")]
    BadSize(usize, std::num::TryFromIntError),

    #[error("payload size was expected but none was found")]
    NoPayloadSize,

    #[error("list size must be known to be serialized")]
    UnknownListSize,

    #[error("unexpected message field {0}")]
    UnexpectedMessageField(&'static str),

    #[error("duplicate message field {0}")]
    DuplicateMessageField(&'static str),

    #[error("missing message field {0}")]
    MissingMessageField(&'static str),

    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct MagicCookie;

impl MagicCookie {
    const VALUE: u32 = 0x42adde42;
}

impl serde::de::Expected for MagicCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl std::fmt::Display for MagicCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", Self::VALUE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_message_to_bytes() {
        use message::*;
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
    fn test_message_from_bytes() {
        let input = &[
            0x42, 0xde, 0xad, 0x42, // cookie
            0xb8, 0x9a, 0x00, 0x00, // id
            0x28, 0x00, 0x00, 0x00, // size
            0xaa, 0x00, 0x02, 0x00, // version, type, flags
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
        let msg = from_bytes::<Message>(input).unwrap();
        use message::*;
        assert_eq!(
            msg,
            Message {
                id: 39608,
                version: 170,
                kind: Kind::Reply,
                flags: Flags::empty(),
                subject: subject::BoundObject::from_values_unchecked(
                    subject::service::Id(39).into(),
                    subject::object::Id(9).into(),
                    action::BoundObject::BoundFunction(104.into()),
                )
                .into(),
                payload: vec![
                    0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d,
                    0x65, 0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35,
                    0x32, 0x2d, 0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30,
                    0x33,
                ],
            }
        );
    }

    #[test]
    fn test_subject_to_bytes() {
        use message::subject::*;
        let subject = BoundObject::from_values_unchecked(
            Service::Other(service::Id(23)),
            Object::Other(object::Id(923)),
            action::BoundObject::BoundFunction(action::Id(392)),
        );
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
}
