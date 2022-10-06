mod de;
mod ser;
mod tuple;

use tuple::Tuple;

//pub enum Type {
//    Void,
//    Bool,
//    Int8,
//    UInt8,
//    Int16,
//    UInt16,
//    Int32,
//    UInt32,
//    Float,
//    Double,
//    String,
//    List(Box<Type>),
//    Map { key: Box<Type>, value: Box<Type> },
//    Object,
//    Tuple(Vec<Type>),
//    Raw,
//    VarArgs(Box<Type>),
//    KwArgs(Box<Type>),
//    Optional(Box<Type>),
//    Dynamic,
//    Unknown,
//}

// TODO: #[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Void,
    Bool(bool),
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float(f32),
    Double(f64),
    String(String),
    List(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Tuple(Tuple),
    Raw(Vec<u8>),
    Optional(Option<Box<Value>>),
}

impl Value {
    pub fn as_tuple(&self) -> Option<&Tuple> {
        if let Self::Tuple(tuple) = self {
            Some(tuple)
        } else {
            None
        }
    }

    pub fn as_tuple_mut(&mut self) -> Option<&mut Tuple> {
        if let Self::Tuple(tuple) = self {
            Some(tuple)
        } else {
            None
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Void
    }
}

impl std::str::FromStr for Value {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Value::String(s.to_string()))
    }
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: serde::Serialize + ?Sized,
{
    value.serialize(ser::Serializer)
}

pub fn from_value<T>(value: Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(value)
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("error: {0}")]
    Custom(String),

    #[error("union types are not supported in the qi type system")]
    UnionAreNotSupported,

    #[error("a map key is missing")]
    MissingMapKey,

    #[error("value cannot be deserialized")]
    ValueCannotBeDeserialized,
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

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
    struct S(S0);

    impl S {
        fn sample() -> (Self, Value) {
            let s = S(S0 {
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
            let t = Value::Tuple(Tuple {
                name: None,
                fields: tuple::Fields::Unnamed(vec![
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
            let s1 = Value::Tuple(Tuple {
                name: Some("S1".to_string()),
                fields: tuple::Fields::Unnamed(vec![
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
            let s0 = Value::Tuple(Tuple {
                name: Some("S0".to_string()),
                fields: vec![
                    tuple::NamedField {
                        name: "t".to_string(),
                        value: t,
                    },
                    tuple::NamedField {
                        name: "r".to_string(),
                        value: r,
                    },
                    tuple::NamedField {
                        name: "o".to_string(),
                        value: o,
                    },
                    tuple::NamedField {
                        name: "s".to_string(),
                        value: s1,
                    },
                    tuple::NamedField {
                        name: "l".to_string(),
                        value: l,
                    },
                    tuple::NamedField {
                        name: "m".to_string(),
                        value: m,
                    },
                ]
                .into(),
            });
            let v = Value::Tuple(Tuple {
                name: Some("S".to_string()),
                fields: vec![s0].into(),
            });
            (s, v)
        }
    }

    #[test]
    fn test_value_from_string() {
        use std::str::FromStr;
        let value = Value::from_str("cookies recipe").unwrap();
        assert_eq!(value, Value::String("cookies recipe".to_string()));
    }

    #[test]
    fn test_value_as_tuple() {
        assert_eq!(
            Value::Tuple(Default::default()).as_tuple(),
            Some(&Tuple::default())
        );
        assert_eq!(Value::Int32(42).as_tuple(), None);
    }

    #[test]
    fn test_value_as_tuple_mut() {
        assert_eq!(
            Value::Tuple(Default::default()).as_tuple_mut(),
            Some(&mut Tuple::default())
        );
        assert_eq!(Value::Int32(42).as_tuple_mut(), None);
    }

    #[test]
    fn test_to_value() {
        let (s, expected) = S::sample();
        let value = to_value(&s).expect("serialization error");
        assert_eq!(value, expected);
    }

    #[test]
    fn test_from_value() {
        let (expected, v) = S::sample();
        let s: S = from_value(v).expect("deserialization error");
        assert_eq!(s, expected);
    }

    #[test]
    fn test_to_from_value_invariant() -> Result<()> {
        let (s, _) = S::sample();
        let s2: S = from_value(to_value(&s)?)?;
        assert_eq!(s, s2);
        Ok(())
    }
}
