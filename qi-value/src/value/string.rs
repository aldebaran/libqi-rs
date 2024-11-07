use super::{IntoValue, Value};
use core::str;
use std::{hash::Hash, string::String as StdString};

#[derive(Debug, Clone, derive_more::From)]
pub enum String<'s> {
    Borrowed(&'s str),
    Owned(StdString),
    Bytes(&'s [u8]),
    ByteBuf(Vec<u8>),
}

impl<'s> String<'s> {
    pub fn from_maybe_utf8(bytes: &'s [u8]) -> Self {
        match str::from_utf8(bytes) {
            Ok(str) => str.into(),
            Err(_) => bytes.into(),
        }
    }

    pub fn from_maybe_utf8_owned(bytes: Vec<u8>) -> Self {
        match StdString::from_utf8(bytes) {
            Ok(str) => str.into(),
            Err(err) => err.into_bytes().into(),
        }
    }

    pub fn into_owned(self) -> String<'static> {
        match self {
            Self::Borrowed(str) => String::Owned(str.to_owned()),
            Self::Owned(str) => String::Owned(str),
            Self::Bytes(bytes) => String::ByteBuf(bytes.to_vec()),
            Self::ByteBuf(bytes) => String::ByteBuf(bytes),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            String::Borrowed(str) => Some(str),
            String::Owned(str) => Some(str),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            String::Borrowed(str) => str.as_bytes(),
            String::Owned(str) => str.as_bytes(),
            String::Bytes(bytes) => bytes,
            String::ByteBuf(bytes) => bytes,
        }
    }
}

impl<'s> std::fmt::Display for String<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Borrowed(str) => str.fmt(f),
            Self::Owned(str) => str.fmt(f),
            Self::Bytes(bytes) => StdString::from_utf8_lossy(bytes).fmt(f),
            Self::ByteBuf(bytes) => StdString::from_utf8_lossy(bytes).fmt(f),
        }
    }
}

impl<'v> From<String<'v>> for Value<'v> {
    fn from(value: String<'v>) -> Self {
        Self::String(value)
    }
}

impl<'s> From<String<'s>> for Vec<u8> {
    fn from(str: String<'s>) -> Self {
        match str {
            String::Borrowed(str) => str.as_bytes().to_owned(),
            String::Owned(str) => str.into_bytes(),
            String::Bytes(bytes) => bytes.to_owned(),
            String::ByteBuf(bytes) => bytes,
        }
    }
}

impl PartialEq<str> for String<'_> {
    fn eq(&self, other: &str) -> bool {
        match self.as_str() {
            Some(str) => str.eq(other),
            None => false,
        }
    }
}

impl PartialEq<String<'_>> for str {
    fn eq(&self, other: &String<'_>) -> bool {
        other.eq(self)
    }
}

impl PartialEq for String<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for String<'_> {}

impl PartialOrd for String<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for String<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl Hash for String<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl<'s> IntoValue<'s> for String<'s> {
    fn into_value(self) -> Value<'s> {
        self.into()
    }
}

impl serde::Serialize for String<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            String::Borrowed(str) => serializer.serialize_str(str),
            String::Owned(str) => serializer.serialize_str(str),
            String::Bytes(bytes) => serializer.serialize_bytes(bytes),
            String::ByteBuf(bytes) => serializer.serialize_bytes(bytes),
        }
    }
}

impl<'s, 'de: 's> serde::Deserialize<'de> for String<'s> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(Visitor)
    }
}

/// A visitor that constructs a string value. Strings values can be non-UTF8 data, so pure bytes are
/// also accepted.
struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = String<'de>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string value")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E>(self, v: StdString) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into())
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into())
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_byte_buf(v.to_vec())
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into())
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into())
    }
}
