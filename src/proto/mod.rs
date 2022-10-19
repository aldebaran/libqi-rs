mod de;
use std::str::Utf8Error;

pub use de::{from_bytes, from_reader, Deserializer};
pub(crate) mod message;
pub(crate) use message::Message;
mod ser;
pub use ser::{to_bytes, to_writer, Serializer};
// TODO: move value outside of proto ?
pub mod value;
pub use value::{from_value, to_value, Value};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

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
    InvalidUtf8(#[from] Utf8Error),

    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
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
        pub fn sample_and_value() -> (Self, Value) {
            let s = Serializable(S0 {
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
            });
            let t = Value::Tuple(value::Tuple {
                name: None,
                fields: value::tuple::Fields::Unnamed(vec![
                    Value::Int8(-8),
                    Value::UInt8(8),
                    Value::Int16(-16),
                    Value::UInt16(16),
                    Value::Int32(-32),
                    Value::UInt32(32),
                    Value::Int64(-64),
                    Value::UInt64(64),
                    Value::Float(32.32),
                    Value::Double(64.64),
                ]),
            });
            let r = Value::Raw(vec![51, 52, 53, 54]);
            let o = Value::Optional(Some(Box::new(Value::Bool(false))));
            let s1 = Value::Tuple(value::Tuple {
                name: Some("S1".to_string()),
                fields: value::tuple::Fields::Unnamed(vec![
                    Value::String("bananas".to_string()),
                    Value::String("oranges".to_string()),
                ]),
            });
            let l = Value::List(vec![
                Value::String("cookies".to_string()),
                Value::String("muffins".to_string()),
            ]);
            let m = Value::Map(vec![
                (Value::Int32(1), Value::String("hello".to_string())),
                (Value::Int32(2), Value::String("world".to_string())),
            ]);
            let s0: Value = value::Tuple {
                name: Some("S0".to_string()),
                fields: vec![
                    value::tuple::NamedField {
                        name: "t".to_string(),
                        value: t,
                    },
                    value::tuple::NamedField {
                        name: "r".to_string(),
                        value: r,
                    },
                    value::tuple::NamedField {
                        name: "o".to_string(),
                        value: o,
                    },
                    value::tuple::NamedField {
                        name: "s".to_string(),
                        value: s1,
                    },
                    value::tuple::NamedField {
                        name: "l".to_string(),
                        value: l,
                    },
                    value::tuple::NamedField {
                        name: "m".to_string(),
                        value: m,
                    },
                ]
                .into(),
            }
            .into();
            let v = Value::Tuple(value::Tuple {
                name: Some("Serializable".to_string()),
                fields: vec![s0].into(),
            });
            (s, v)
        }
    }

    #[test]
    fn test_to_from_bytes_invariant() {
        let (sample, _) = Serializable::sample_and_value();
        let bytes = to_bytes(&sample).unwrap();
        let sample2: Serializable = from_bytes(&bytes).unwrap();
        assert_eq!(sample, sample2);
    }
}
