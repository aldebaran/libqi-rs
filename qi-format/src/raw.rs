use crate::Error;
use derive_more::{AsRef, Index, IndexMut, Into};
use std::borrow::Cow;

/// [`Raw`] represents a `raw` value in the `qi` type system.
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
    Default, Clone, Into, PartialEq, Eq, PartialOrd, Ord, Index, IndexMut, AsRef, Hash, Debug,
)]
#[into(owned, ref, ref_mut)]
#[as_ref(forward)]
pub struct Raw<'r>(Cow<'r, [u8]>);

impl<'r> Raw<'r> {
    /// Constructs an empty raw value.
    ///
    /// The resulting raw value is equal to the result of converting an empty slice of bytes to a
    /// raw value.
    ///
    /// # Example:
    ///
    /// ```
    /// # use qi_format::Raw;
    /// assert_eq!(Raw::new(), Raw::from(&[]));
    /// ```
    pub fn new() -> Self {
        Self(Cow::default())
    }

    /// Constructs a raw value from a slice of bytes.
    ///
    /// # Example
    ///
    /// ```
    /// # use qi_format::Raw;
    /// assert_eq!(Raw::from_bytes(&[1, 2, 3]).as_ref(),
    ///            &[1, 2, 3]);
    /// ```
    pub fn from_bytes(bytes: &'r [u8]) -> Self {
        Self(Cow::Borrowed(bytes))
    }

    /// Constructs a raw value from a buffer of bytes.
    ///
    /// # Example
    ///
    /// ```
    /// # use qi_format::Raw;
    /// assert_eq!(Raw::from_byte_buf(vec![1, 2, 3]).as_ref(),
    ///            &[1, 2, 3]);
    /// ```
    pub fn from_byte_buf(buf: Vec<u8>) -> Self {
        Self(Cow::Owned(buf))
    }

    /// Returns the slice of bytes that compose the raw value.
    ///
    /// # Example
    /// ```
    /// # use qi_format::Raw;
    /// assert_eq!(Raw::from(&[1, 2, 3]).as_bytes(),
    ///            &[1, 2, 3]);
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns true if the raw value data is borrowed.
    ///
    /// # Example
    /// ```
    /// # use qi_format::Raw;
    /// assert!(Raw::from_bytes(&[1, 2, 3]).is_borrowed());
    /// assert!(!Raw::from_byte_buf(vec![1, 2, 3]).is_borrowed());
    /// ```
    pub fn is_borrowed(&self) -> bool {
        match &self.0 {
            Cow::Borrowed(_) => true,
            Cow::Owned(_) => false,
        }
    }

    /// Converts the raw value into one that owns its data.
    ///
    /// # Example
    /// ```
    /// # use qi_format::Raw;
    /// let owned_buf = vec![1, 2, 3];
    /// let borrowed_raw = Raw::from_bytes(&owned_buf);
    /// let owned_raw = borrowed_raw.clone().into_owned();
    ///
    /// assert_eq!(borrowed_raw, [1, 2, 3]);
    /// assert_eq!(owned_raw, [1, 2, 3]);
    ///
    /// // Dropping the source buffer that owns the original data.
    /// drop(owned_buf);
    ///
    /// // assert_eq!(borrowed_raw, [1, 2, 3]); // error: borrowing a dropped value.
    /// assert_eq!(owned_raw, [1, 2, 3]); // no problem, this one owns its data.
    /// ```
    pub fn into_owned(self) -> Raw<'static> {
        Raw(Cow::Owned(self.0.into_owned()))
    }

    pub fn as_borrowed_bytes(&self) -> Option<&'r [u8]> {
        match self.0 {
            Cow::Borrowed(b) => Some(b),
            Cow::Owned(_) => None,
        }
    }

    pub fn into_byte_buf(self) -> Vec<u8> {
        match self.0 {
            Cow::Borrowed(b) => Vec::from(b),
            Cow::Owned(b) => b,
        }
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

impl<'r> From<&'r Raw<'r>> for &'r [u8] {
    fn from(r: &'r Raw<'r>) -> Self {
        r.as_ref()
    }
}

impl<'r> From<Raw<'r>> for Vec<u8> {
    fn from(r: Raw<'r>) -> Self {
        r.into_byte_buf()
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

impl<'r> std::fmt::Display for Raw<'r> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0.iter() {
            write!(f, "\\x{byte:x}")?;
        }
        Ok(())
    }
}

impl<'r> serde::Serialize for Raw<'r> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de, 'r> serde::Deserialize<'de> for Raw<'r>
where
    'de: 'r,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Raw<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a raw value")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Raw::from_byte_buf(v.to_owned()))
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Raw::from_bytes(v))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Raw::from_byte_buf(v))
            }
        }
        deserializer.deserialize_byte_buf(Visitor)
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
    use super::*;
    use assert_matches::assert_matches;
    use serde::de::{Deserialize, IntoDeserializer};

    #[test]
    fn test_raw_serde() {
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &Raw::from_bytes(&[1, 2, 3, 4, 5]),
            &[Token::BorrowedBytes(&[1, 2, 3, 4, 5])],
        );
        assert_tokens(
            &Raw::from_bytes(&[1, 2, 3, 4, 5]),
            &[Token::ByteBuf(&[1, 2, 3, 4, 5])],
        );
        assert_tokens(
            &Raw::from_bytes(&[1, 2, 3, 4, 5]),
            &[Token::Bytes(&[1, 2, 3, 4, 5])],
        );
    }

    #[test]
    fn test_raw_deserializer() {
        use serde_bytes::{ByteBuf, Bytes};
        assert_matches!(
            {
                let s = Raw::from_bytes(&[1, 2, 3, 4]);
                <&[u8]>::deserialize(s.into_deserializer())
            },
            Ok([1, 2, 3, 4])
        );
        assert_matches!(
            {
                let s = Raw::from_bytes(&[1, 2, 3, 4]);
                <&Bytes>::deserialize(s.into_deserializer())
            },
            Ok(b) => assert_eq!(b, &[1, 2, 3, 4])
        );
        assert_matches!(
            {
                let s = Raw::from_byte_buf(vec![1, 2, 3, 4]);
                <Vec<u8>>::deserialize(s.into_deserializer())
            },
            Ok(v) => assert_eq!(v, [1, 2, 3, 4])
        );
        assert_matches!(
            {
                let s = Raw::from_byte_buf(vec![1, 2, 3, 4]);
                <ByteBuf>::deserialize(s.into_deserializer())
            },
            Ok(buf) => assert_eq!(buf, [1, 2, 3, 4])
        );
    }

    #[test]
    fn test_raw_deserializer_ref() {
        let s = &Raw::from_bytes(&[1, 2, 3]);
        assert_matches!(<&[u8]>::deserialize(s.into_deserializer()), Ok([1, 2, 3]));
        let s = &Raw::from_byte_buf(vec![1, 2, 3]);
        assert_matches!(<&[u8]>::deserialize(s.into_deserializer()), Ok([1, 2, 3]));
    }
}
