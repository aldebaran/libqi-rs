// TODO: #![deny(missing_docs)]

mod de;
mod ser;
mod signature;
mod r#type;
pub mod value;

#[doc(inline)]
pub use r#type::Type;

#[doc(inline)]
pub use signature::Signature;

#[doc(inline)]
pub use value::{from_borrowed_value, from_value, to_value, AnnotatedValue, Value};

#[doc(inline)]
pub use ser::{to_bytes, to_writer, Serializer};

#[doc(inline)]
pub use de::{from_bytes, from_reader, Deserializer};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("the type of the data is unknown (the `qi` format is not self-describing)")]
    UnknownDataType,

    #[error("size conversion failed: {0}")]
    BadSize(std::num::TryFromIntError),

    #[error("payload size was expected but none was found")]
    NoPayloadSize,

    #[error("list size must be known to be serialized")]
    UnknownListSize,

    #[error("unexpected message field {0}")]
    UnexpectedMessageField(&'static str),

    #[error("duplicate message field {0}")]
    DuplicateMessageField(&'static str),

    #[error("missing message field {0}")]
    MissingMessageField(&'static str),

    #[error("string data is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),

    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct S0 {
        t: (i8, u8, i16, u16, i32, u32, i64, u64, f32, f64),
        #[serde(with = "serde_bytes")]
        r: Vec<u8>,
        o: Option<bool>,
        s: S1,
        l: Vec<String>,
        m: BTreeMap<i32, String>,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct S1(String, String);

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    pub struct Serializable(S0);

    impl Serializable {
        pub fn sample() -> Self {
            Self(S0 {
                t: (-8, 8, -16, 16, -32, 32, -64, 64, 32.32, 64.64),
                r: vec![51, 52, 53, 54],
                o: Some(false),
                s: S1("bananas".to_string(), "oranges".to_string()),
                l: vec!["cookies".to_string(), "muffins".to_string()],
                m: {
                    let mut m = BTreeMap::new();
                    m.insert(1, "hello".to_string());
                    m.insert(2, "world".to_string());
                    m
                },
            })
        }

        pub fn sample_as_value() -> Value<'static> {
            use value::*;
            let t = Value::Tuple(Tuple::new(vec![
                Value::Int8(-8),
                Value::UnsignedInt8(8),
                Value::Int16(-16),
                Value::UnsignedInt16(16),
                Value::Int32(-32),
                Value::UnsignedInt32(32),
                Value::Int64(-64),
                Value::UnsignedInt64(64),
                Value::Float32(32.32),
                Value::Float64(64.64),
            ]));
            let r = Value::Raw(vec![51, 52, 53, 54].into());
            let o = Value::Option(Some(Value::Bool(false).into()));
            let s1 = Tuple::new(vec![Value::from("bananas"), Value::from("oranges")]);
            let s = Value::from(s1);
            let l = Value::List(vec![Value::from("cookies"), Value::from("muffins")]);
            let m = Value::Map(Map::from(vec![
                (Value::Int32(1), Value::String("hello".into())),
                (Value::Int32(2), Value::String("world".into())),
            ]));
            let s0 = Value::from(Tuple::new(vec![t, r, o, s, l, m]));
            Value::from(Tuple::new(vec![s0]))
        }
    }

    #[test]
    fn test_to_from_bytes_serializable() {
        let sample = Serializable::sample();
        let bytes = to_bytes(&sample).unwrap();
        let sample2: Serializable = from_bytes(&bytes).unwrap();
        assert_eq!(sample, sample2);
    }

    #[test]
    fn test_to_from_bytes_annotated_value() {
        let value_before = AnnotatedValue::new(Serializable::sample_as_value());
        let bytes = to_bytes(&value_before).unwrap();
        let value_after: AnnotatedValue = from_bytes(&bytes).unwrap();
        assert_eq!(value_before, value_after);
    }

    #[test]
    fn test_option_i32_to_bytes() {
        assert_eq!(
            to_bytes(&Some(42)).unwrap(),
            vec![0x01, 0x2a, 0x00, 0x00, 0x00]
        );
        assert_eq!(to_bytes(&Option::<i32>::None).unwrap(), vec![0x00]);
    }

    // Tuple size is not prepended.
    #[test]
    fn test_tuple_to_bytes() {
        assert_eq!(
            to_bytes(&(42u16, "str", true)).unwrap(),
            vec![
                42, 0, // u16
                3, 0, 0, 0, 0x73, 0x74, 0x72, // string, prepended with its length
                1     // bool
            ]
        );
    }

    #[test]
    fn test_option_char_from_bytes() {
        assert_eq!(
            from_bytes::<Option<char>>(&[0x01, 0x01, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63]).unwrap(),
            Some('a')
        );
        assert_eq!(
            from_bytes::<Option<char>>(&[0x00, 0x01, 0x02, 0x03, 0x04]).unwrap(),
            None,
        );
    }
}
