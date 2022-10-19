mod de;
use std::str::Utf8Error;

pub use de::{from_bytes, from_message, from_reader, Deserializer};
pub mod message;
pub use message::Message;
mod ser;
pub use ser::{to_bytes, to_message, to_writer, Serializer};
// TODO: move value outside of proto ?
pub mod value;
pub use value::Value;

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

// TODO: test using ser/de `value::tests::S`
