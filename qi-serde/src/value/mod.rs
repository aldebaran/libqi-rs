pub mod de;
pub mod ser;
use crate::Type;
pub use de::{from_value, from_value_ref};
use indexmap::IndexMap;
pub use ser::to_value;

/// Any value supported by the qi type system.
// TODO: #[non_exhaustive]
// TODO: Enable the value to borrow data from sources.
// TODO: Implement PartialOrd manually.
#[derive(Default, Clone, PartialEq, Debug)]
pub enum Value {
    #[default]
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
    Raw(Vec<u8>),
    Option(Option<Box<Value>>),
    List(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Tuple(Vec<Value>),
    TupleStruct {
        name: String,
        elements: Vec<Value>,
    },
    Struct {
        name: String,
        // IndexMap to preserve insertion order.
        fields: IndexMap<String, Value>,
    },
    // TODO: Handle enumerations
}

fn keys(map: &Vec<(Value, Value)>) -> impl Iterator<Item = &Value> {
    map.iter().map(|(k, _v)| k)
}

fn values(map: &Vec<(Value, Value)>) -> impl Iterator<Item = &Value> {
    map.iter().map(|(_k, v)| v)
}

fn iter_common_value_type<'v, I>(iter: I) -> Type
where
    I: IntoIterator<Item = &'v Value>,
{
    let mut iter = iter.into_iter();
    let mut common_type = None;
    loop {
        match iter.next() {
            Some(elem) => match common_type {
                None => common_type = Some(elem.get_type()),
                Some(t) => match elem.get_type().common_type(&t) {
                    Some(t) => common_type = Some(t),
                    None => {
                        // Different types in the list, therefore the value type is deduced as
                        // dynamic.
                        break Type::Dynamic;
                    }
                },
            },
            None => match common_type {
                // No item in the list, the value type is deduced as dynamic.
                None => break Type::Dynamic,
                // Only one item in the list, the value type is deduced as dynamic.
                Some(t) => break t,
            },
        }
    }
}

impl Value {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_string().map(|s| s.as_str())
    }

    pub fn as_tuple(&self) -> Option<&Vec<Value>> {
        if let Self::Tuple(elements) = self {
            Some(elements)
        } else {
            None
        }
    }

    pub fn as_tuple_mut(&mut self) -> Option<&mut Vec<Value>> {
        if let Self::Tuple(elements) = self {
            Some(elements)
        } else {
            None
        }
    }

    pub fn into<T>(&self) -> Result<T, de::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        from_value_ref(self)
    }

    pub fn get_type(&self) -> Type {
        match self {
            Value::Void => Type::Void,
            Value::Bool(_) => Type::Bool,
            Value::Int8(_) => Type::Int8,
            Value::UInt8(_) => Type::UInt8,
            Value::Int16(_) => Type::UInt16,
            Value::UInt16(_) => Type::UInt16,
            Value::Int32(_) => Type::Int32,
            Value::UInt32(_) => Type::UInt32,
            Value::Int64(_) => Type::UInt32,
            Value::UInt64(_) => Type::UInt64,
            Value::Float(_) => Type::Float,
            Value::Double(_) => Type::Double,
            Value::String(_) => Type::String,
            Value::Raw(_) => Type::Raw,
            Value::Option(option) => Type::Option(Box::new(match option {
                Some(value) => value.get_type(),
                None => Type::Dynamic,
            })),
            Value::List(list) => Type::List(Box::new(iter_common_value_type(list))),
            Value::Map(map) => Type::Map {
                key: Box::new(iter_common_value_type(keys(map))),
                value: Box::new(iter_common_value_type(values(map))),
            },
            Value::Tuple(elements) => Type::Tuple(elements.iter().map(Self::get_type).collect()),
            Value::TupleStruct { name, elements } => Type::TupleStruct {
                name: name.clone(),
                elements: elements.iter().map(Self::get_type).collect(),
            },
            Value::Struct { name, fields } => Type::Struct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(name, value)| (name.clone(), value.get_type()))
                    .collect(),
            },
        }
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl TryFrom<Value> for String {
    type Error = TryFromValueError;
    fn try_from(d: Value) -> Result<Self, Self::Error> {
        match d {
            Value::String(s) => Ok(s),
            _ => Err(TryFromValueError),
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.into())
    }
}

impl<'v> TryFrom<&'v Value> for &'v str {
    type Error = TryFromValueError;
    fn try_from(d: &'v Value) -> Result<Self, Self::Error> {
        d.as_str().ok_or(TryFromValueError)
    }
}

// TODO: Implement all conversions

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[error("value conversion failed")]
pub struct TryFromValueError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::*, Reflect};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_value_from_string() {
        assert_eq!(
            Value::from("muffins recipe".to_owned()),
            Value::String("muffins recipe".into())
        );
    }

    #[test]
    fn test_value_try_into_string() {
        let res: Result<String, _> = Value::String("muffins recipe".into()).try_into();
        assert_eq!(res, Ok("muffins recipe".to_owned()));
        let res: Result<String, _> = Value::Int32(321).try_into();
        assert_eq!(res, Err(TryFromValueError));
    }

    #[test]
    fn test_value_from_str() {
        assert_eq!(
            Value::from("cookies recipe"),
            Value::String("cookies recipe".into())
        );
    }

    #[test]
    fn test_value_try_into_str() {
        let value = Value::String("muffins recipe".into());
        let res: Result<&str, _> = (&value).try_into();
        assert_eq!(res, Ok("muffins recipe"));
        let res: Result<&str, _> = (&Value::Int32(321)).try_into();
        assert_eq!(res, Err(TryFromValueError));
    }

    #[test]
    fn test_value_as_string() {
        assert_eq!(
            Value::from("muffins").as_string(),
            Some(&"muffins".to_owned())
        );
        assert_eq!(Value::Int32(321).as_string(), None);
    }

    #[test]
    fn test_value_as_str() {
        assert_eq!(Value::from("cupcakes").as_str(), Some("cupcakes"));
        assert_eq!(Value::Float(3.15).as_str(), None);
    }

    #[test]
    fn test_value_as_tuple() {
        assert_eq!(Value::Tuple(Vec::new()).as_tuple(), Some(&Vec::new()));
        assert_eq!(Value::Int32(42).as_tuple(), None);
    }

    #[test]
    fn test_value_as_tuple_mut() {
        assert_eq!(
            Value::Tuple(Vec::new()).as_tuple_mut(),
            Some(&mut Vec::new())
        );
        assert_eq!(Value::Int32(42).as_tuple_mut(), None);
    }

    #[test]
    fn test_to_value() {
        let s = Serializable::sample();
        let expected = Serializable::sample_as_value();
        let value = to_value(&s, Serializable::get_type()).unwrap();
        assert_eq!(value, expected);
    }

    #[test]
    fn test_value_to_value() {
        let src_value = Serializable::sample_as_value();
        let value = to_value(&src_value, Serializable::get_type()).unwrap();
        assert_eq!(value, src_value);
    }

    #[test]
    fn test_from_value() {
        let expected = Serializable::sample();
        let value = Serializable::sample_as_value();
        let s: Serializable = from_value_ref(&value).unwrap();
        assert_eq!(s, expected);
    }

    #[test]
    fn test_value_from_value() {
        let src_value = Serializable::sample_as_value();
        let value: Value = from_value_ref(&src_value).unwrap();
        assert_eq!(value, src_value);
    }

    #[test]
    fn test_to_from_value_invariant() {
        let src_s = Serializable::sample();
        let s: Serializable =
            from_value_ref(&to_value(&src_s, Serializable::get_type()).unwrap()).unwrap();
        assert_eq!(s, src_s);
    }

    #[test]
    fn test_ser_de() {
        use indexmap::indexmap;
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &Value::Struct {
                name: "S".into(),
                fields: indexmap![
                    "l".into() => Value::List(vec![ Value::String("cookies".into()), Value::String("muffins".into()), ]),
                    "r".into() => Value::Raw(vec![1, 2, 3, 4]),
                    "i".into() => Value::Int32(12),
                    "om".into() => Value::Option(Some(
                            Value::Map(vec![ ( Value::String("pi".into()), Value::Float(std::f32::consts::PI),), ( Value::String("tau".into()), Value::Float(std::f32::consts::TAU),), ],)
                            .into(),
                        ),
                    ),
                    "ol".into() => Value::Option(None),
                ],
            },
            &[
                Token::Tuple { len: 2 },
                Token::Str("([s]ri+{sf}+l)<S,l,r,i,om,ol>"),
                Token::Tuple { len: 5 },
                Token::Seq { len: Some(2) },
                Token::Str("cookies"),
                Token::Str("muffins"),
                Token::SeqEnd,
                Token::Bytes(&[1, 2, 3, 4]),
                Token::I32(12),
                Token::Some,
                Token::Map { len: Some(1) },
                Token::Str("pi"),
                Token::F32(std::f32::consts::PI),
                Token::Str("tau"),
                Token::F32(std::f32::consts::TAU),
                Token::MapEnd,
                Token::None,
                Token::TupleEnd,
                Token::TupleEnd,
            ],
        )
    }
}
