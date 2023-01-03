use crate::Error;
use derive_more::{AsRef, Index, IndexMut, Into};
use derive_new::new;
use std::borrow::Cow;

#[derive(
    new, Default, Clone, Into, PartialEq, Eq, PartialOrd, Ord, Index, IndexMut, AsRef, Hash, Debug,
)]
#[into(owned, ref, ref_mut)]
#[as_ref(forward)]
pub struct Raw<'r>(pub(crate) Cow<'r, [u8]>);

impl<'r> Raw<'r> {
    pub fn from_bytes(bytes: &'r [u8]) -> Self {
        Self(Cow::Borrowed(bytes))
    }

    pub fn from_byte_buf(buf: Vec<u8>) -> Self {
        Self(Cow::Owned(buf))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl<'r> From<&'r [u8]> for Raw<'r> {
    fn from(bytes: &'r [u8]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl<'r> From<Vec<u8>> for Raw<'r> {
    fn from(buf: Vec<u8>) -> Self {
        Self::from_byte_buf(buf)
    }
}

impl<'r> serde::Serialize for Raw<'r> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

impl<'de> serde::Deserialize<'de> for Raw<'de> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

impl<'de> serde::de::Deserializer<'de> for Raw<'de> {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char str string bytes byte_buf option unit
        tuple unit_struct tuple_struct struct newtype_struct
        seq map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Cow::Borrowed(b) => visitor.visit_borrowed_bytes(b),
            Cow::Owned(b) => visitor.visit_byte_buf(b),
        }
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for Raw<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> serde::de::Deserializer<'de> for &'de Raw<'de> {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char str string bytes byte_buf option unit
        tuple unit_struct tuple_struct struct newtype_struct
        seq map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(&self.0)
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for &'de Raw<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_raw_deserializer() {
        todo!()
    }
}
