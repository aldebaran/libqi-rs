// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

const FALSE_BOOL: u8 = 0;
const TRUE_BOOL: u8 = 1;

mod read;

mod write;

pub mod ser;
#[doc(inline)]
pub use ser::{to_bytes, Serializer};

pub mod de;
#[doc(inline)]
pub use de::{from_buf, Deserializer};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("short read")]
    ShortRead,

    #[error("the value '{0}' is not a `bool` value")]
    NotABoolValue(u8),

    #[error("cannot deserialize any data, the type information of the expected value is required (the `qi` format is not self-describing)")]
    CannotDeserializeAny,

    #[error("size conversion error")]
    SizeConversionError(std::num::TryFromIntError),

    #[error("list and maps size must be known to be serialized")]
    UnspecifiedListMapSize,

    #[error("expected {0} elements, got one more")]
    UnexpectedElement(usize),

    #[error("string data is not valid UTF-8")]
    InvalidStringUtf8(#[from] std::str::Utf8Error),

    #[error("{0}")]
    Custom(std::string::String),
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::InvalidStringUtf8(err.utf8_error())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
