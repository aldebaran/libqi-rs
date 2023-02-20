// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

mod num_bool;
#[doc(inline)]
pub use num_bool::{
    Bool, Float32, Float64, Int16, Int32, Int64, Int8, Number, UInt16, UInt32, UInt64, UInt8,
};

mod string;
#[doc(inline)]
pub use string::String;

mod raw;
#[doc(inline)]
pub use raw::Raw;

mod tuple;
#[doc(inline)]
pub use tuple::{Tuple, Unit};

// The module is not named `type` because it is a keyword.
mod typing;
#[doc(inline)]
pub use typing::{
    Tuple as TypeTuple, TupleAnnotations as TypeAnnotations,
    TupleAnnotationsError as TypeAnnotationsError, Type,
};

mod map;
pub use map::Map;

mod signature;
#[doc(inline)]
pub use signature::Signature;

pub mod value;
#[doc(inline)]
pub use value::{List, Option, Value};

pub mod dynamic;
#[doc(inline)]
pub use dynamic::Dynamic;

pub mod object;
#[doc(inline)]
pub use object::{Object, MetaObject, MetaMethod, MetaSignal, MetaProperty};

mod read;
#[doc(inline)]
pub use read::*;

mod write;
#[doc(inline)]
pub use write::*;

pub mod ser;
#[doc(inline)]
pub use ser::*;

pub mod de;
#[doc(inline)]
pub use de::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("the value '{0}' is not a `bool` value")]
    NotABoolValue(u8),

    #[error("cannot deserialize any data, the type information of the expected value is required (the `qi` format is not self-describing)")]
    CannotDeserializeAny,

    #[error("size conversion error: {0}")]
    SizeConversionError(std::num::TryFromIntError),

    #[error("list and maps size must be known to be serialized")]
    UnspecifiedListMapSize,

    #[error("expected {0} elements, got one more")]
    UnexpectedElement(usize),

    #[error("string data is not valid UTF-8: {0}")]
    InvalidStringUtf8(#[from] std::str::Utf8Error),

    #[error("{0}")]
    Custom(std::string::String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests;
