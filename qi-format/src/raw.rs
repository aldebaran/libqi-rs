use crate::Error;
use derive_more::{AsRef, Index, IndexMut, Into};
use derive_new::new;
use std::borrow::Cow;

/// A `qi` raw value.
///
/// It is a value composed of entities external to the `qi` type system, that needs further
/// interpretation.
///
/// # Lifetime and data ownership
///
/// This type borrows data when possible, avoiding copies when unneeded. It may however own its
/// data when required (see the [`is_borrowed`](Self::is_borrowed) and
/// [`into_owned`](Self::into_owned) member functions).
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

    pub fn is_borrowed(&self) -> bool {
        match &self.0 {
            Cow::Borrowed(_) => true,
            Cow::Owned(_) => false,
        }
    }

    pub fn into_owned(self) -> Raw<'static> {
        Raw(Cow::Owned(self.0.into_owned()))
    }
}

impl<'r, const N: usize> From<&'r [u8; N]> for Raw<'r> {
    fn from(bytes: &'r [u8; N]) -> Self {
        Self::from_bytes(bytes)
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

impl<'r> PartialEq<[u8]> for Raw<'r> {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_bytes() == other
    }
}

impl<'r, const N: usize> PartialEq<[u8; N]> for Raw<'r> {
    fn eq(&self, other: &[u8; N]) -> bool {
        self.as_bytes() == other
    }
}

impl<'r1, 'r2> PartialEq<&'r1 [u8]> for Raw<'r2> {
    fn eq(&self, other: &&'r1 [u8]) -> bool {
        self.as_bytes() == *other
    }
}

impl<'r1, 'r2, const N: usize> PartialEq<&'r1 [u8; N]> for Raw<'r2> {
    fn eq(&self, other: &&'r1 [u8; N]) -> bool {
        self.as_bytes() == *other
    }
}

impl<'r> PartialEq<Raw<'r>> for [u8] {
    fn eq(&self, other: &Raw<'r>) -> bool {
        self == other.as_bytes()
    }
}

impl<'r, const N: usize> PartialEq<Raw<'r>> for [u8; N] {
    fn eq(&self, other: &Raw<'r>) -> bool {
        self == other.as_bytes()
    }
}

impl<'r1, 'r2> PartialEq<Raw<'r1>> for &'r2 [u8] {
    fn eq(&self, other: &Raw<'r1>) -> bool {
        *self == other.as_bytes()
    }
}

impl<'r1, 'r2, const N: usize> PartialEq<Raw<'r1>> for &'r2 [u8; N] {
    fn eq(&self, other: &Raw<'r1>) -> bool {
        *self == other.as_bytes()
    }
}

impl<'r> IntoIterator for &'r Raw<'r> {
    /// Raw iterator iterates over bytes.
    type Item = &'r u8;

    /// Raw iterator is an iterator over a slice of bytes.
    type IntoIter = std::slice::Iter<'r, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
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
        map enum identifier ignored_any
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

    // Allows deserializing into Vec<T> where T is deserializable from a byte.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        use serde::de::value::SeqDeserializer;
        let seq_deserializer: SeqDeserializer<_, Error> =
            SeqDeserializer::new(self.into_iter().copied());
        seq_deserializer.deserialize_seq(visitor)
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
        map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(&self.0)
    }

    // Allows deserializing into Vec<T> where T is deserializable from a byte.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        use serde::de::value::SeqDeserializer;
        let seq_deserializer: SeqDeserializer<_, Error> =
            SeqDeserializer::new(self.into_iter().copied());
        seq_deserializer.deserialize_seq(visitor)
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
