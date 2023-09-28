use std::fmt::Display;

use crate::{from_value, to_value, Result};
use bytes::Bytes;

/// A formatted `qi` value.
#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Value {
    data: Bytes,
}

impl Value {
    pub fn new() -> Self {
        Self { data: Bytes::new() }
    }

    pub fn from_bytes(data: Bytes) -> Self {
        Self { data }
    }

    pub fn as_bytes(&self) -> &Bytes {
        &self.data
    }

    pub fn to_bytes(&self) -> Bytes {
        self.data.clone()
    }

    pub fn bytes_len(&self) -> usize {
        self.data.len()
    }

    pub fn from_serializable<T>(s: &T) -> Result<Self>
    where
        T: serde::Serialize,
    {
        to_value(s)
    }

    pub fn to_deserializable<'v, T>(&'v self) -> Result<T>
    where
        T: serde::Deserialize<'v>,
    {
        from_value(self)
    }
}

impl<const N: usize> From<[u8; N]> for Value {
    fn from(bytes: [u8; N]) -> Self {
        Self::from_bytes(Bytes::copy_from_slice(bytes.as_slice()))
    }
}

impl From<&'static [u8]> for Value {
    fn from(bytes: &'static [u8]) -> Self {
        Self::from_bytes(Bytes::from_static(bytes))
    }
}

impl From<Bytes> for Value {
    fn from(value: Bytes) -> Self {
        Value::from_bytes(value)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bytes(len={})", self.data.len())
    }
}
