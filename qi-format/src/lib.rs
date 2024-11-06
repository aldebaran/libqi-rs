// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

const FALSE_BOOL: u8 = 0;
const TRUE_BOOL: u8 = 1;

pub mod read;
pub mod write;

pub mod ser;
#[doc(inline)]
pub use ser::{to_bytes, BytesSerializer};

pub mod de;
#[doc(inline)]
pub use de::{from_slice, SliceDeserializer};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("short read")]
    ShortRead,

    #[error("the value '{0}' is not a `bool` value")]
    NotABoolValue(u8),

    #[error("unknown element")]
    UnknownElement,

    #[error("failure to convert size")]
    SizeConversionError(std::num::TryFromIntError),

    #[error("list and maps size must be known to be serialized")]
    MissingSequenceSize,

    #[error("expected {0} elements, got one more")]
    UnexpectedElement(usize),

    #[error("failure to process sequence size")]
    SequenceSize(#[source] Box<Error>),

    #[error("failure to process a sequence item at {name}")]
    SequenceElement { name: String, source: Box<Error> },

    #[error("failure to process a map element key at index {index}")]
    MapKey { index: usize, source: Box<Error> },

    #[error("failure to process a map element value at index {index}")]
    MapValue { index: usize, source: Box<Error> },

    #[error("failure to process a variant index")]
    VariantIndex(#[source] Box<Error>),

    #[error("{0}")]
    Custom(std::string::String),
}

pub type Result<T> = std::result::Result<T, Error>;
