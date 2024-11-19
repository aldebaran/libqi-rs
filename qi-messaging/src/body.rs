use bytes::{Buf, Bytes};
use std::marker::PhantomData;

pub trait Body: Sized {
    type Error: serde::de::Error;
    type Data: Buf;

    fn from_bytes(bytes: Bytes) -> Result<Self, Self::Error>;

    fn into_data(self) -> Result<Self::Data, Self::Error>;

    fn serialize<T>(value: &T) -> Result<Self, Self::Error>
    where
        T: serde::Serialize;

    fn deserialize_seed<'de, T>(&'de self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>;

    fn deserialize<'de, T>(&'de self) -> Result<T, Self::Error>
    where
        T: serde::de::Deserialize<'de>,
    {
        self.deserialize_seed(PhantomData)
    }
}
