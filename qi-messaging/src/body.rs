use crate::value;
use bytes::{Buf, Bytes};

pub trait BodyBuf: Sized {
    type Error;
    type Data: Buf;
    type Deserializer<'de>: serde::Deserializer<'de>
    where
        Self: 'de;

    fn from_bytes(bytes: Bytes) -> Result<Self, Self::Error>;

    fn into_data(self) -> Result<Self::Data, Self::Error>;

    fn serialize<T>(value: &T) -> Result<Self, Self::Error>
    where
        T: serde::Serialize;

    fn deserializer(&mut self) -> Self::Deserializer<'_>;

    fn deserialize_value<'v>(
        &'v mut self,
        ty: Option<&value::Type>,
    ) -> Result<value::Value<'v>, Self::Error>
    where
        <Self::Deserializer<'v> as serde::Deserializer<'v>>::Error: Into<Self::Error>,
    {
        use serde::de::DeserializeSeed;
        value::de::ValueOfType::new(ty)
            .deserialize(self.deserializer())
            .map_err(Into::into)
    }
}
