use crate::{format, messaging};
use bytes::Bytes;
pub use qi_value::*;

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BinaryFormattedValue(Bytes);

impl BinaryFormattedValue {
    #[cfg(test)]
    pub(super) fn from_static(bytes: &'static [u8]) -> Self {
        Self(Bytes::from_static(bytes))
    }
}

impl messaging::Body for BinaryFormattedValue {
    type Error = format::Error;
    type Data = Bytes;

    fn from_bytes(bytes: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(bytes))
    }

    fn into_data(self) -> Result<Self::Data, Self::Error> {
        Ok(self.0)
    }

    fn serialize<T>(value: &T) -> Result<Self, Self::Error>
    where
        T: serde::Serialize,
    {
        format::to_bytes(value).map(Self)
    }

    fn deserialize_seed<'de, T>(&'de self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut format::SliceDeserializer::new(&self.0))
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error(transparent)]
    FromValue(#[from] value::FromValueError),

    #[error(transparent)]
    Format(#[from] format::Error),
}
