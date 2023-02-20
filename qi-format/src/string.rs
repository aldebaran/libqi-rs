mod private {
    use std::borrow::Cow;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub enum String<'s> {
        Utf8(Cow<'s, str>),
        NonUtf8(Cow<'s, [u8]>),
    }
}

use crate::{Error, Raw};
use std::borrow::Cow;

/// [`String`] represents a `string` value in the `qi` type system.
///
/// It is a sequence of characters, with no constraint on the character set.
///
/// Consequently, it is represented as a slice of bytes, which means that this type is able to
/// implement [`AsRef<u8>`].
///
/// # Lifetime and data ownership
///
/// This type borrows data when possible, avoiding copies when unneeded. It may however own its
/// data when required (see the [`is_borrowed`](Self::is_borrowed) and
/// [`into_owned`](Self::into_owned) member functions).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct String<'s>(private::String<'s>);

impl<'s> String<'s> {
    /// Constructs an empty string.
    ///
    /// The resulting string is equal to the result of converting an empty string literal to
    /// a string.
    ///
    /// # Example:
    ///
    /// ```
    /// # use qi_format::String;
    /// assert_eq!(String::new(), String::from(""));
    /// ```
    pub fn new() -> Self {
        Self(private::String::Utf8(Default::default()))
    }

    /// Constructs a string from a slice of bytes.
    ///
    /// # Example
    ///
    /// ```
    /// # use qi_format::String;
    /// assert_eq!(String::from_bytes(&[97, 98, 99]).as_ref(),
    ///            &[97, 98, 99]);
    /// ```
    pub fn from_bytes(bytes: &'s [u8]) -> Self {
        use private::String;
        match std::str::from_utf8(bytes) {
            Ok(s) => Self(String::Utf8(Cow::Borrowed(s))),
            Err(_) => Self(String::NonUtf8(Cow::Borrowed(bytes))),
        }
    }

    /// Constructs a string from a buffer of bytes.
    ///
    /// # Example
    ///
    /// ```
    /// # use qi_format::String;
    /// assert_eq!(String::from_byte_buf(vec![97, 98, 99]).as_ref(),
    ///            &[97, 98, 99]);
    /// ```
    pub fn from_byte_buf(buf: Vec<u8>) -> Self {
        use private::String;
        match std::string::String::from_utf8(buf) {
            Ok(s) => Self(String::Utf8(Cow::Owned(s))),
            Err(e) => Self(String::NonUtf8(Cow::Owned(e.into_bytes()))),
        }
    }

    /// Constructs a string from a borrowed string.
    ///
    /// # Example
    ///
    /// ```
    /// # use qi_format::String;
    /// assert_eq!(String::from_borrowed_str("abc").as_ref(),
    ///            &[97, 98, 99]);
    /// ```
    pub fn from_borrowed_str(str: &'s str) -> Self {
        use private::String;
        Self(String::Utf8(Cow::Borrowed(str)))
    }

    /// Constructs a string from an owned standard string.
    ///
    /// # Example
    ///
    /// ```
    /// # use qi_format::String;
    /// let s = std::string::String::from("abc");
    /// assert_eq!(String::from_string(s).as_ref(),
    ///            &[97, 98, 99]);
    /// ```
    pub fn from_string(str: std::string::String) -> Self {
        use private::String;
        Self(String::Utf8(Cow::Owned(str)))
    }

    /// Returns the slice of bytes that compose the string.
    ///
    /// # Example
    /// ```
    /// # use qi_format::String;
    /// assert_eq!(String::from("abc").as_bytes(),
    ///            &[97, 98, 99]);
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        use private::String;
        match &self.0 {
            String::Utf8(s) => s.as_bytes(),
            String::NonUtf8(b) => b,
        }
    }

    /// Returns true if the string data is borrowed.
    ///
    /// # Example
    /// ```
    /// # use qi_format::String;
    /// assert!(String::from_borrowed_str("abc").is_borrowed());
    /// assert!(String::from_bytes(&[0, 159, 146, 150]).is_borrowed());
    /// assert!(!String::from_string("abc".to_owned()).is_borrowed());
    /// assert!(!String::from_byte_buf(vec![0, 159, 146, 150]).is_borrowed());
    /// ```
    pub fn is_borrowed(&self) -> bool {
        use private::String;
        match &self.0 {
            String::Utf8(s) => match s {
                Cow::Borrowed(_) => true,
                Cow::Owned(_) => false,
            },
            String::NonUtf8(b) => match b {
                Cow::Borrowed(_) => true,
                Cow::Owned(_) => false,
            },
        }
    }

    /// Converts the string value into one that owns its data.
    ///
    /// # Example
    /// ```
    /// # use qi_format::String;
    /// let owned_std_str = std::string::String::from("abc");
    /// let borrowed_str = String::from_borrowed_str(&owned_std_str);
    /// let owned_str = borrowed_str.clone().into_owned();
    ///
    /// assert_eq!(borrowed_str, "abc");
    /// assert_eq!(owned_str, "abc");
    ///
    /// // Dropping the source string that owns the original data.
    /// drop(owned_std_str);
    ///
    /// // assert_eq!(borrowed_str, "abc"); error: borrowing a dropped value.
    /// assert_eq!(owned_str, "abc"); // no problem, this one owns its data.
    /// ```
    pub fn into_owned(self) -> String<'static> {
        use private::String;
        String(match self.0 {
            String::Utf8(s) => String::Utf8(Cow::Owned(s.into_owned())),
            String::NonUtf8(b) => String::NonUtf8(Cow::Owned(b.into_owned())),
        })
    }
}

/// Strings can be manipulated as slices of bytes.
///
/// # Example
/// ```
/// # use qi_format::String;
/// let s = String::from_bytes(&[0, 159, 146, 150]);
/// assert_eq!(s.as_ref(), [0, 159, 146, 150]);
/// ```
impl<'s> AsRef<[u8]> for String<'s> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// Default construction for a string creates an empty string.
///
/// # Example
///
/// ```
/// # use qi_format::String;
/// assert_eq!(String::default(), String::from(""));
/// ```
impl<'s> Default for String<'s> {
    fn default() -> Self {
        Self::new()
    }
}

/// Conversion from a slice of bytes into a string.
///
/// # Example
///
/// ```
/// # use qi_format::String;
/// assert_eq!(String::from(&[1, 2][..]).as_bytes(), &[1, 2]);
/// ```
impl<'s> From<&'s [u8]> for String<'s> {
    fn from(s: &'s [u8]) -> Self {
        Self::from_bytes(s)
    }
}

/// Conversion from a buffer of bytes into a string.
///
/// # Example
///
/// ```
/// # use qi_format::String;
/// assert_eq!(String::from(vec![1, 2]).as_bytes(), &[1, 2]);
/// ```
impl<'s> From<Vec<u8>> for String<'s> {
    fn from(buf: Vec<u8>) -> Self {
        Self::from_byte_buf(buf)
    }
}

/// Conversion from a borrowed string into a string.
///
/// # Example
///
/// ```
/// # use qi_format::String;
/// assert_eq!(String::from("abc").as_bytes(), &[97, 98, 99]);
/// ```
impl<'s> From<&'s str> for String<'s> {
    fn from(s: &'s str) -> Self {
        Self::from_borrowed_str(s)
    }
}

/// Conversion from an owned standard string into a string.
///
/// # Example
///
/// ```
/// # use qi_format::String;
/// assert_eq!(String::from("abc".to_string()).as_bytes(), &[97, 98, 99]);
/// ```
impl<'s> From<std::string::String> for String<'s> {
    fn from(s: std::string::String) -> Self {
        Self::from_string(s)
    }
}

/// Conversion from a raw value into a string.
///
/// # Example
///
/// ```
/// # use qi_format::{String, Raw};
/// let raw = Raw::from_bytes(&[97, 98, 99]);
/// let str = String::from(raw);
/// assert!(str.is_borrowed());
/// assert_eq!(str, "abc");
/// ```
impl<'s> From<Raw<'s>> for String<'s> {
    fn from(r: Raw<'s>) -> Self {
        match r.as_borrowed_bytes() {
            Some(b) => Self::from_bytes(b),
            None => Self::from_byte_buf(r.into_byte_buf()),
        }
    }
}

/// Conversion from a string into a raw value.
///
/// # Example
///
/// ```
/// # use qi_format::{String, Raw};
/// let str = String::from("abc");
/// let raw = Raw::from(str);
/// assert!(raw.is_borrowed());
/// assert_eq!(raw, [97, 98, 99]);
/// ```
impl<'s> From<String<'s>> for Raw<'s> {
    fn from(s: String<'s>) -> Self {
        use private::String;
        match s.0 {
            String::Utf8(str) => match str {
                Cow::Borrowed(str) => Self::from_bytes(str.as_bytes()),
                Cow::Owned(str) => Self::from_byte_buf(str.into_bytes()),
            },
            String::NonUtf8(bytes) => match bytes {
                Cow::Borrowed(bytes) => Self::from_bytes(bytes),
                Cow::Owned(buf) => Self::from_byte_buf(buf),
            },
        }
    }
}

impl<'s> PartialEq<str> for String<'s> {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'s1, 's2> PartialEq<&'s1 str> for String<'s2> {
    fn eq(&self, other: &&'s1 str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'s> PartialEq<String<'s>> for str {
    fn eq(&self, other: &String<'s>) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'s1, 's2> PartialEq<String<'s1>> for &'s2 str {
    fn eq(&self, other: &String<'s1>) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

/// Converts the string into an iterator on its bytes.
///
/// # Example
///
/// ```
/// # use qi_format::String;
/// let s = String::from("abc");
/// let mut iter = s.into_iter();
/// assert_eq!(iter.next(), Some(&97));
/// assert_eq!(iter.next(), Some(&98));
/// assert_eq!(iter.next(), Some(&99));
/// assert_eq!(iter.next(), None);
/// ```
impl<'s> IntoIterator for &'s String<'s> {
    /// String iterator iterates over bytes.
    type Item = &'s u8;

    /// String iterator is an iterator over a slice of bytes.
    type IntoIter = std::slice::Iter<'s, u8>;

    fn into_iter(self) -> Self::IntoIter {
        use private::String;
        let bytes = match &self.0 {
            String::Utf8(s) => s.as_bytes(),
            String::NonUtf8(b) => b,
        };
        bytes.iter()
    }
}

/// Formats a string. If the string is not UTF-8, it is formatted as a bytestring.
///
/// # Example
///
/// ```
/// # use qi_format::String;
/// assert_eq!(format!("{}", String::from_borrowed_str("abc")), "abc");
/// assert_eq!(format!("{}", String::from_bytes(&[0, 159, 146, 150])), "\\x00\\x9f\\x92\\x96");
/// ```
impl<'s> std::fmt::Display for String<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use private::String;
        match &self.0 {
            String::Utf8(s) => s.fmt(f),
            String::NonUtf8(bytes) => {
                for byte in bytes.iter() {
                    write!(f, "\\x{byte:0>2x}")?;
                }
                Ok(())
            }
        }
    }
}

impl<'s> serde::Serialize for String<'s> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use private::String;
        match &self.0 {
            String::Utf8(s) => serializer.serialize_str(s),
            String::NonUtf8(b) => serializer.serialize_bytes(b),
        }
    }
}

impl<'de, 's> serde::Deserialize<'de> for String<'s>
where
    'de: 's,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = String<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(String::from_string(v.to_owned()))
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(String::from_borrowed_str(v))
            }

            fn visit_string<E>(self, v: std::string::String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(String::from_string(v))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(String::from_byte_buf(v.to_owned()))
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(String::from_bytes(v))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(String::from_byte_buf(v))
            }
        }
        deserializer.deserialize_string(Visitor)
    }
}

impl<'de> serde::de::Deserializer<'de> for String<'de> {
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
        use private::String;
        match self.0 {
            String::Utf8(s) => match s {
                Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                Cow::Owned(s) => visitor.visit_string(s),
            },
            String::NonUtf8(s) => match s {
                Cow::Borrowed(s) => visitor.visit_borrowed_bytes(s),
                Cow::Owned(s) => visitor.visit_byte_buf(s),
            },
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

impl<'de> serde::de::IntoDeserializer<'de, Error> for String<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> serde::de::Deserializer<'de> for &'de String<'de> {
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
        use private::String;
        match &self.0 {
            String::Utf8(s) => visitor.visit_borrowed_str(s),
            String::NonUtf8(b) => visitor.visit_borrowed_bytes(b),
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

impl<'de> serde::de::IntoDeserializer<'de, Error> for &'de String<'de> {
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
    fn test_string_eq() {
        assert_eq!(
            String::from_borrowed_str("abc"),
            String::from_string("abc".to_owned())
        );
        assert_eq!(
            String::from_bytes(&[0, 159, 146, 150]),
            String::from_byte_buf(vec![0, 159, 146, 150])
        );
        assert_eq!(
            String::from_bytes(&[97, 98, 99]),
            String::from_borrowed_str("abc")
        );
        assert_ne!(
            String::from_borrowed_str("abc"),
            String::from_borrowed_str("def")
        );
        assert_ne!(
            String::from_bytes(&[0, 159, 146, 150]),
            String::from_bytes(&[0, 159, 146, 151])
        );
    }

    // String serializes into and deserializes from strings or bytes in any form.
    #[test]
    fn test_string_serde() {
        use serde_test::{assert_tokens, Token};
        assert_tokens(&String::from("abc"), &[Token::BorrowedStr("abc")]);
        assert_tokens(&String::from("abc"), &[Token::String("abc")]);
        assert_tokens(&String::from("abc"), &[Token::Str("abc")]);
        assert_tokens(
            &String::from_bytes(&[0, 159, 146, 150]),
            &[Token::BorrowedBytes(&[0, 159, 146, 150])],
        );
        assert_tokens(
            &String::from_bytes(&[0, 159, 146, 150]),
            &[Token::ByteBuf(&[0, 159, 146, 150])],
        );
        assert_tokens(
            &String::from_bytes(&[0, 159, 146, 150]),
            &[Token::Bytes(&[0, 159, 146, 150])],
        );
    }

    // Deserialization borrows data when possible.
    #[test]
    fn test_string_deserialize_borrows() {
        use serde::de::value::{BorrowedBytesDeserializer, BorrowedStrDeserializer};

        let str = "abc";
        let deserializer = BorrowedStrDeserializer::new(str);
        let s: Result<_, serde::de::value::Error> = String::deserialize(deserializer);
        assert!(s.unwrap().is_borrowed());

        let bytes = &[1u8, 2u8, 3u8];
        let deserializer = BorrowedBytesDeserializer::new(bytes);
        let s: Result<_, serde::de::value::Error> = String::deserialize(deserializer);
        assert!(s.unwrap().is_borrowed());
    }

    #[test]
    fn test_string_deserializer() {
        use serde::de::IntoDeserializer;
        use serde_bytes::{ByteBuf, Bytes};
        assert_matches!(
            {
                let s = String::from_borrowed_str("abc");
                <&str>::deserialize(s.into_deserializer())
            },
            Ok("abc")
        );
        assert_matches!(
            {
                let s = String::from_string("abc".to_owned());
                <std::string::String>::deserialize(s.into_deserializer())
            },
            Ok(s) => assert_eq!(s, "abc")
        );
        assert_matches!(
            {
                let s = String::from_bytes(&[0, 159, 146, 150]);
                <&[u8]>::deserialize(s.into_deserializer())
            },
            Ok([0, 159, 146, 150])
        );
        assert_matches!(
            {
                let s = String::from_bytes(&[0, 159, 146, 150]);
                <&Bytes>::deserialize(s.into_deserializer())
            },
            Ok(b) => assert_eq!(b, &[0, 159, 146, 150])
        );
        assert_matches!(
            {
                let s = String::from_byte_buf(vec![0, 159, 146, 150]);
                <Vec<u8>>::deserialize(s.into_deserializer())
            },
            Ok(v) => assert_eq!(v, [0, 159, 146, 150])
        );
        assert_matches!(
            {
                let s = String::from_byte_buf(vec![0, 159, 146, 150]);
                <ByteBuf>::deserialize(s.into_deserializer())
            },
            Ok(buf) => assert_eq!(buf, [0, 159, 146, 150])
        );
    }

    #[test]
    fn test_string_deserializer_ref() {
        let s = &String::from_borrowed_str("abc");
        assert_matches!(<&str>::deserialize(s.into_deserializer()), Ok("abc"));
        let s = &String::from_string("abc".to_owned());
        assert_matches!(<&str>::deserialize(s.into_deserializer()), Ok("abc"));
        let s = &String::from_bytes(&[0, 159, 146, 150]);
        assert_matches!(
            <&[u8]>::deserialize(s.into_deserializer()),
            Ok([0, 159, 146, 150])
        );
        let s = &String::from_byte_buf(vec![0, 159, 146, 150]);
        assert_matches!(
            <&[u8]>::deserialize(s.into_deserializer()),
            Ok([0, 159, 146, 150])
        );
    }
}
