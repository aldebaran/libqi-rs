pub mod de;
pub mod ser;
use crate::Type;
pub use de::{from_any_value, from_any_value_ref};
use indexmap::IndexMap;
pub use ser::to_any_value;

/// Any value supported by the qi type system.
// TODO: #[non_exhaustive]
// TODO: Enable the value to borrow data from sources.
// TODO: Implement PartialOrd manually.
#[derive(Default, Clone, PartialEq, Debug)]
pub enum AnyValue {
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
    Option {
        value_type: Type,
        option: Option<Box<AnyValue>>,
    },
    List {
        value_type: Type,
        list: Vec<AnyValue>,
    },
    Map {
        value_type: Type,
        key_type: Type,
        map: Vec<(AnyValue, AnyValue)>,
    },
    Tuple(Vec<AnyValue>),
    TupleStruct {
        name: String,
        elements: Vec<AnyValue>,
    },
    Struct {
        name: String,
        fields: IndexMap<String, AnyValue>,
    },
    // TODO: Handle enumerations
}

impl AnyValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            AnyValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_string().map(|s| s.as_str())
    }

    pub fn as_tuple(&self) -> Option<&Vec<AnyValue>> {
        if let Self::Tuple(elements) = self {
            Some(elements)
        } else {
            None
        }
    }

    pub fn as_tuple_mut(&mut self) -> Option<&mut Vec<AnyValue>> {
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
        from_any_value_ref(self)
    }

    pub fn runtime_type(&self) -> Type {
        match self {
            AnyValue::Void => Type::Void,
            AnyValue::Bool(_) => Type::Bool,
            AnyValue::Int8(_) => Type::Int8,
            AnyValue::UInt8(_) => Type::UInt8,
            AnyValue::Int16(_) => Type::UInt16,
            AnyValue::UInt16(_) => Type::UInt16,
            AnyValue::Int32(_) => Type::Int32,
            AnyValue::UInt32(_) => Type::UInt32,
            AnyValue::Int64(_) => Type::UInt32,
            AnyValue::UInt64(_) => Type::UInt64,
            AnyValue::Float(_) => Type::Float,
            AnyValue::Double(_) => Type::Double,
            AnyValue::String(_) => Type::String,
            AnyValue::Raw(_) => Type::Raw,
            AnyValue::Option { value_type, .. } => Type::Option(value_type.clone().into()),
            AnyValue::List { value_type, .. } => Type::List(value_type.clone().into()),
            AnyValue::Map {
                key_type,
                value_type,
                ..
            } => Type::Map {
                key: key_type.clone().into(),
                value: value_type.clone().into(),
            },
            AnyValue::Tuple(elements) => {
                Type::Tuple(elements.iter().map(Self::runtime_type).collect())
            }
            AnyValue::TupleStruct { name, elements } => Type::TupleStruct {
                name: name.clone(),
                elements: elements.iter().map(Self::runtime_type).collect(),
            },
            AnyValue::Struct { name, fields } => Type::Struct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(name, value)| (name.clone(), value.runtime_type()))
                    .collect(),
            },
        }
    }

    pub fn matches_type(&self, _t: &Type) -> bool {
        todo!()
    }
}

impl From<String> for AnyValue {
    fn from(s: String) -> Self {
        AnyValue::String(s)
    }
}

impl TryFrom<AnyValue> for String {
    type Error = TryFromValueError;
    fn try_from(d: AnyValue) -> Result<Self, Self::Error> {
        match d {
            AnyValue::String(s) => Ok(s),
            _ => Err(TryFromValueError),
        }
    }
}

impl From<&str> for AnyValue {
    fn from(s: &str) -> Self {
        AnyValue::String(s.into())
    }
}

impl<'v> TryFrom<&'v AnyValue> for &'v str {
    type Error = TryFromValueError;
    fn try_from(d: &'v AnyValue) -> Result<Self, Self::Error> {
        d.as_str().ok_or(TryFromValueError)
    }
}

// TODO: Implement all conversions

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[error("any value conversion failed")]
pub struct TryFromValueError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::*, Value};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_anyvalue_from_string() {
        assert_eq!(
            AnyValue::from("muffins recipe".to_owned()),
            AnyValue::String("muffins recipe".into())
        );
    }

    #[test]
    fn test_anyvalue_try_into_string() {
        let res: Result<String, _> = AnyValue::String("muffins recipe".into()).try_into();
        assert_eq!(res, Ok("muffins recipe".to_owned()));
        let res: Result<String, _> = AnyValue::Int32(321).try_into();
        assert_eq!(res, Err(TryFromValueError));
    }

    #[test]
    fn test_anyvalue_from_str() {
        assert_eq!(
            AnyValue::from("cookies recipe"),
            AnyValue::String("cookies recipe".into())
        );
    }

    #[test]
    fn test_anyvalue_try_into_str() {
        let value = AnyValue::String("muffins recipe".into());
        let res: Result<&str, _> = (&value).try_into();
        assert_eq!(res, Ok("muffins recipe"));
        let res: Result<&str, _> = (&AnyValue::Int32(321)).try_into();
        assert_eq!(res, Err(TryFromValueError));
    }

    #[test]
    fn test_anyvalue_as_string() {
        assert_eq!(
            AnyValue::from("muffins").as_string(),
            Some(&"muffins".to_owned())
        );
        assert_eq!(AnyValue::Int32(321).as_string(), None);
    }

    #[test]
    fn test_anyvalue_as_str() {
        assert_eq!(AnyValue::from("cupcakes").as_str(), Some("cupcakes"));
        assert_eq!(AnyValue::Float(3.15).as_str(), None);
    }

    #[test]
    fn test_anyvalue_as_tuple() {
        assert_eq!(AnyValue::Tuple(Vec::new()).as_tuple(), Some(&Vec::new()));
        assert_eq!(AnyValue::Int32(42).as_tuple(), None);
    }

    #[test]
    fn test_anyvalue_as_tuple_mut() {
        assert_eq!(
            AnyValue::Tuple(Vec::new()).as_tuple_mut(),
            Some(&mut Vec::new())
        );
        assert_eq!(AnyValue::Int32(42).as_tuple_mut(), None);
    }

    #[test]
    fn test_to_anyvalue() {
        let s = Serializable::sample();
        let expected = Serializable::sample_as_value();
        let value = to_any_value(&s, Serializable::get_type()).unwrap();
        assert_eq!(value, expected);
    }

    #[test]
    fn test_anyvalue_to_anyvalue() {
        let src_value = Serializable::sample_as_value();
        let value = to_any_value(&src_value, Serializable::get_type()).unwrap();
        assert_eq!(value, src_value);
    }

    #[test]
    fn test_from_anyvalue() {
        let expected = Serializable::sample();
        let value = Serializable::sample_as_value();
        let s: Serializable = from_any_value_ref(&value).unwrap();
        assert_eq!(s, expected);
    }

    #[test]
    fn test_anyvalue_from_anyvalue() {
        let src_value = Serializable::sample_as_value();
        let value: AnyValue = from_any_value_ref(&src_value).unwrap();
        assert_eq!(value, src_value);
    }

    #[test]
    fn test_to_from_anyvalue_invariant() {
        let src_s = Serializable::sample();
        let s: Serializable =
            from_any_value_ref(&to_any_value(&src_s, Serializable::get_type()).unwrap()).unwrap();
        assert_eq!(s, src_s);
    }

    #[test]
    fn test_anyvalue_ser_de() {
        use indexmap::indexmap;
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &AnyValue::Struct {
                name: "S".into(),
                fields: indexmap![
                    "l".into() => AnyValue::List {
                        value_type: Type::String,
                        list: vec![
                            AnyValue::String("cookies".into()),
                            AnyValue::String("muffins".into()),
                        ],
                    },
                    "r".into() => AnyValue::Raw(vec![1, 2, 3, 4]),
                    "i".into() => AnyValue::Int32(12),
                    "om".into() => AnyValue::Option {
                        value_type: Type::Map {
                            key: Type::String.into(),
                            value: Type::Float.into(),
                        },
                        option: Some(
                            AnyValue::Map {
                                key_type: Type::String,
                                value_type: Type::Float,
                                map: vec![
                                    (
                                        AnyValue::String("pi".into()),
                                        AnyValue::Float(std::f32::consts::PI),
                                    ),
                                    (
                                        AnyValue::String("tau".into()),
                                        AnyValue::Float(std::f32::consts::TAU),
                                    ),
                                ],
                            }
                            .into(),
                        ),
                    },
                    "ol".into() => AnyValue::Option {
                        value_type: Type::Int64,
                        option: None,
                    },
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
