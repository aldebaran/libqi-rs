mod private {
    use std::borrow::Cow;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub enum String<'s> {
        Utf8(Cow<'s, str>),
        NonUtf8(Cow<'s, [u8]>),
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct String<'s>(private::String<'s>);

impl<'s> AsRef<[u8]> for String<'s> {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            private::String::Utf8(s) => s.as_bytes(),
            private::String::NonUtf8(s) => s,
        }
    }
}

impl<'s> std::ops::Deref for String<'s> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'s> Default for String<'s> {
    fn default() -> Self {
        Self(private::String::Utf8(Default::default()))
    }
}

impl<'s, I> std::ops::Index<I> for String<'s>
where
    [u8]: std::ops::Index<I>,
{
    type Output = <[u8] as std::ops::Index<I>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        todo!()
    }
}

impl<'s> From<&'s [u8]> for String<'s> {
    fn from(s: &'s [u8]) -> Self {
        use private::String;
        use std::borrow::Cow;
        match std::str::from_utf8(s) {
            Ok(s) => Self(String::Utf8(Cow::Borrowed(s))),
            Err(_) => Self(String::NonUtf8(Cow::Borrowed(s))),
        }
    }
}

impl<'s> From<Vec<u8>> for String<'s> {
    fn from(s: Vec<u8>) -> Self {
        use private::String;
        use std::borrow::Cow;
        match std::string::String::from_utf8(s) {
            Ok(s) => Self(String::Utf8(Cow::Owned(s))),
            Err(e) => Self(String::NonUtf8(Cow::Owned(e.into_bytes()))),
        }
    }
}

impl<'s> From<&'s str> for String<'s> {
    fn from(s: &'s str) -> Self {
        use private::String;
        use std::borrow::Cow;
        Self(String::Utf8(Cow::Borrowed(s)))
    }
}

impl<'s> From<std::string::String> for String<'s> {
    fn from(s: std::string::String) -> Self {
        use private::String;
        use std::borrow::Cow;
        Self(String::Utf8(Cow::Owned(s)))
    }
}

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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

// The derived implementation of `Deserialize` for `Cow` never borrows.
// We define an custom implementation instead.
impl<'de> serde::Deserialize<'de> for String<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

impl<'de> serde::de::Deserializer<'de> for String<'de> {
    type Error = serde::de::value::Error;

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

impl<'de> serde::de::IntoDeserializer<'de, serde::de::value::Error> for String<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> serde::de::Deserializer<'de> for &'de String<'de> {
    type Error = serde::de::value::Error;

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
        match &self.0 {
            String::Utf8(s) => match s {
                Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                Cow::Owned(s) => visitor.visit_str(s),
            },
            String::NonUtf8(s) => match s {
                Cow::Borrowed(s) => visitor.visit_borrowed_bytes(s),
                Cow::Owned(s) => visitor.visit_bytes(s),
            },
        }
    }
}

impl<'de> serde::de::IntoDeserializer<'de, serde::de::value::Error> for &'de String<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_bytes, to_bytes};
    use assert_matches::assert_matches;

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
    fn test_string_to_format_bytes() {
        let s = String::from("sample data");
        let bytes = to_bytes(&s).unwrap();
        assert_eq!(
            bytes,
            [
                0x0b, 0x00, 0x00, 0x00, // size
                0x73, 0x61, 0x6d, 0x70, 0x6c, 0x65, // 'sample'
                0x20, 0x64, 0x61, 0x74, 0x61, // ' data'
            ]
        );
    }

    #[test]
    fn test_string_from_format_bytes() {
        let bytes = [
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        assert_matches!(from_bytes::<String>(&bytes), Ok(s) => s == String::from("s"));
    }
}
