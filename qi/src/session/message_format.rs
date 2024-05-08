use crate::{format, messaging};
use bytes::Bytes;

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BinaryFormattedBody(pub Bytes);

impl messaging::BodyBuf for BinaryFormattedBody {
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

    fn deserialize<T>(self) -> Result<T, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        format::from_buf(self.0)
    }
}
