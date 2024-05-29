use crate::{format, messaging, Error};
use bytes::Bytes;

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BinaryValue(pub Bytes);

impl messaging::BodyBuf for BinaryValue {
    type Error = Error;
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
        Ok(format::to_bytes(value).map(Self)?)
    }

    fn deserialize<'de, T>(&'de self) -> Result<T, Self::Error>
    where
        T: serde::de::Deserialize<'de>,
    {
        Ok(format::from_buf(self.0.as_ref())?)
    }
}
