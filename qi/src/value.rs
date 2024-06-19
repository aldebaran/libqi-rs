use crate::{format, messaging, Error, Result};
use bytes::Bytes;
pub use qi_value::*;

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(super) struct BinaryValue(Bytes);

impl BinaryValue {
    #[cfg(test)]
    pub(super) fn from_static(bytes: &'static [u8]) -> Self {
        Self(Bytes::from_static(bytes))
    }

    pub(super) fn deserialize_value_of_type<'v>(
        &'v mut self,
        ty: Option<&Type>,
    ) -> Result<Value<'v>> {
        use messaging::BodyBuf;
        use serde::de::DeserializeSeed;
        de::ValueOfType::new(ty)
            .deserialize(self.deserializer())
            .map_err(Into::into)
    }

    #[cfg(test)]
    pub(crate) fn deserialize_value<'v, T>(&'v mut self) -> Result<T>
    where
        T: Reflect + FromValue<'v>,
    {
        Ok(self
            .deserialize_value_of_type(T::ty().as_ref())?
            .cast_into()?)
    }
}

impl messaging::BodyBuf for BinaryValue {
    type Error = Error;
    type Data = Bytes;
    type Deserializer<'de> = format::Deserializer<'de, Bytes>
    where
        Self: 'de;

    fn from_bytes(bytes: Bytes) -> Result<Self> {
        Ok(Self(bytes))
    }

    fn into_data(self) -> Result<Self::Data> {
        Ok(self.0)
    }

    fn serialize<T>(value: &T) -> Result<Self>
    where
        T: serde::Serialize,
    {
        Ok(format::to_bytes(value).map(Self)?)
    }

    fn deserializer(&mut self) -> Self::Deserializer<'_> {
        format::Deserializer::from_buf(&mut self.0)
    }
}
