mod private {
    use std::borrow::Cow;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub enum String<'s> {
        Utf8(Cow<'s, str>),
        NonUtf8(Cow<'s, [u8]>),
    }
}

use crate::{Error, Raw};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct String<'s>(private::String<'s>);

impl<'s> String<'s> {
    pub fn new() -> Self {
        Self(private::String::Utf8(Default::default()))
    }

    pub fn from_bytes(bytes: &'s [u8]) -> Self {
        use private::String;
        use std::borrow::Cow;
        match std::str::from_utf8(bytes) {
            Ok(s) => Self(String::Utf8(Cow::Borrowed(s))),
            Err(_) => Self(String::NonUtf8(Cow::Borrowed(bytes))),
        }
    }

    pub fn from_byte_buf(buf: Vec<u8>) -> Self {
        use private::String;
        use std::borrow::Cow;
        match std::string::String::from_utf8(buf) {
            Ok(s) => Self(String::Utf8(Cow::Owned(s))),
            Err(e) => Self(String::NonUtf8(Cow::Owned(e.into_bytes()))),
        }
    }

    pub fn from_borrowed_str(str: &'s str) -> Self {
        use private::String;
        use std::borrow::Cow;
        Self(String::Utf8(Cow::Borrowed(str)))
    }

    pub fn from_string(str: std::string::String) -> Self {
        use private::String;
        use std::borrow::Cow;
        Self(String::Utf8(Cow::Owned(str)))
    }

    pub fn as_bytes(&self) -> &[u8] {
        use private::String;
        match &self.0 {
            String::Utf8(s) => s.as_bytes(),
            String::NonUtf8(b) => b,
        }
    }

    pub fn into_owned(self) -> String<'static> {
        use private::String;
        use std::borrow::Cow;
        String(match self.0 {
            String::Utf8(s) => String::Utf8(Cow::Owned(s.into_owned())),
            String::NonUtf8(b) => String::NonUtf8(Cow::Owned(b.into_owned())),
        })
    }
}

impl<'s> AsRef<[u8]> for String<'s> {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            private::String::Utf8(s) => s.as_bytes(),
            private::String::NonUtf8(s) => s,
        }
    }
}

impl<'s> Default for String<'s> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'s> From<&'s [u8]> for String<'s> {
    fn from(s: &'s [u8]) -> Self {
        Self::from_bytes(s)
    }
}

impl<'s> From<Vec<u8>> for String<'s> {
    fn from(buf: Vec<u8>) -> Self {
        Self::from_byte_buf(buf)
    }
}

impl<'s> From<&'s str> for String<'s> {
    fn from(s: &'s str) -> Self {
        Self::from_borrowed_str(s)
    }
}

impl<'s> From<std::string::String> for String<'s> {
    fn from(s: std::string::String) -> Self {
        Self::from_string(s)
    }
}

impl<'s> From<Raw<'s>> for String<'s> {
    fn from(r: Raw<'s>) -> Self {
        use std::borrow::Cow;
        match r.0 {
            Cow::Borrowed(bytes) => Self::from_bytes(bytes),
            Cow::Owned(buf) => Self::from_byte_buf(buf),
        }
    }
}

impl<'s> From<String<'s>> for Raw<'s> {
    fn from(s: String<'s>) -> Self {
        use private::String;
        use std::borrow::Cow;
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

// TODO: PartialEq + Eq String with str

impl<'s> IntoIterator for &'s String<'s> {
    type Item = &'s u8;
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

impl<'s> serde::Serialize for String<'s> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

impl<'de> serde::Deserialize<'de> for String<'de> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
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
        seq map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        use private::String;
        use std::borrow::Cow;
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
        seq map enum identifier ignored_any
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
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for &'de String<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_string_from_bytes() {
        todo!()
    }

    #[test]
    fn test_string_from_byte_buf() {
        todo!()
    }

    #[test]
    fn test_string_from_str() {
        todo!()
    }

    #[test]
    fn test_string_from_string() {
        todo!()
    }

    #[test]
    fn test_string_into_iterator() {
        todo!()
    }

    #[test]
    fn test_string_as_ref() {
        todo!()
    }

    #[test]
    fn test_string_serde() {
        todo!()
    }

    #[test]
    fn test_string_deserializer() {
        todo!()
    }
}
