use crate::{format, messaging, value, Error, Result};
use bytes::{Buf, Bytes};

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(super) struct BinaryValue(Bytes);

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

pub(super) fn deserialize_reflect<'b, T, B>(buf: &'b mut B) -> Result<T>
where
    T: value::FromValue<'b> + value::Reflect,
    B: Buf,
{
    let value: value::Value<'b> = format::from_buf(buf)?;
    Ok(T::from_value(value)?)
}
