// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

const FALSE_BOOL: u8 = 0;
const TRUE_BOOL: u8 = 1;

mod value;
#[doc(inline)]
pub use value::Value;

mod read;

mod write;

pub mod ser;
#[doc(inline)]
pub use ser::{to_value, Serializer};

pub mod de;
#[doc(inline)]
pub use de::{from_value, Deserializer};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("input/output error")]
    Io(#[from] std::io::Error),

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

    #[error("string data \"{0}\" is not valid UTF-8")]
    InvalidStringUtf8(String, #[source] std::str::Utf8Error),

    #[error("{0}")]
    Custom(std::string::String),
}

pub type Result<T> = std::result::Result<T, Error>;
