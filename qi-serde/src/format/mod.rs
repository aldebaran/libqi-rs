mod de;
mod ser;

pub use de::{from_bytes, from_message, from_reader, Deserializer};
pub use ser::{to_bytes, to_message, to_writer, Serializer};
use std::str::Utf8Error;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("size conversion failed: {0}")]
    BadSize(std::num::TryFromIntError),

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

    #[error("string data is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] Utf8Error),

    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{message::*, tests::*, Message, Value};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_to_from_bytes_serializable() {
        let sample = Serializable::sample();
        let bytes = to_bytes(&sample).unwrap();
        let sample2: Serializable = from_bytes(&bytes).unwrap();
        assert_eq!(sample, sample2);
    }

    #[test]
    fn test_to_from_bytes_value() {
        todo!()
    }

    #[test]
    fn test_to_from_bytes_value_as_dynamic() {
        todo!()
    }

    #[test]
    fn test_dynamic_value_from_message() {
        let input = vec![
            0x42, 0xde, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            // payload
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let message: Message = from_reader(input.as_slice()).unwrap();
        assert_eq!(
            message,
            Message {
                id: 990340,
                version: 0,
                kind: Kind::Error,
                flags: Flags::empty(),
                subject: Subject::try_from_values(
                    subject::service::Id(47),
                    subject::object::Id(1),
                    subject::action::Id(178)
                )
                .unwrap(),
                payload: vec![
                    0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20,
                    0x72, 0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20,
                    0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64
                ],
            }
        );
        let dynamic: Value = from_message(&message).unwrap();
        assert_eq!(dynamic, Value::from("The robot is not localized"));
    }
}
