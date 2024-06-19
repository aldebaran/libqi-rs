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
}
