mod de;
mod ser;
use crate::typesystem::Type;
pub use de::from_any_value;
pub use ser::to_any_value;

/// Any value supported by the qi type system.
// TODO: #[non_exhaustive]
// TODO: Enable the value to borrow data from sources.
#[derive(Default, Clone, PartialEq, PartialOrd, Debug)]
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
    Tuple(Tuple),
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

    pub fn into<T>(&self) -> Result<T, de::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        from_any_value(self)
    }

    pub fn get_type(&self) -> Type {
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
            AnyValue::Option { value_type, .. } => Type::option(value_type.clone()),
            AnyValue::List { value_type, .. } => Type::list(value_type.clone()),
            AnyValue::Map {
                key_type,
                value_type,
                ..
            } => Type::map(key_type.clone(), value_type.clone()),
            AnyValue::Tuple(t) => t.get_type(),
        }
    }
}

pub mod tuple {
    use super::{AnyValue, Type};
    use crate::typesystem::tuple;
    pub type Tuple = tuple::Tuple<AnyValue>;
    pub type Elements = tuple::Elements<AnyValue>;
    pub type Field = tuple::Field<AnyValue>;

    impl tuple::Tuple<AnyValue> {
        pub fn get_type(&self) -> Type {
            let (name, elements) = (self.name, self.elements);
            let elements = elements.into_iter().map(|v| v.get_type());
            let elements = elements.collect();
            use crate::typesystem::r#type::Tuple as TypeTuple;
            TypeTuple { name, elements }.into()
        }
    }
}
pub use tuple::Tuple;

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

impl From<Tuple> for AnyValue {
    fn from(t: Tuple) -> Self {
        AnyValue::Tuple(t)
    }
}

impl TryFrom<AnyValue> for Tuple {
    type Error = TryFromValueError;

    fn try_from(d: AnyValue) -> Result<Self, Self::Error> {
        match d {
            AnyValue::Tuple(t) => Ok(t),
            _ => Err(TryFromValueError),
        }
    }
}

// TODO: Implement all conversions

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[error("dynamic value conversion failed")]
pub struct TryFromValueError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_dynamic_from_string() {
        assert_eq!(
            AnyValue::from("muffins recipe".to_owned()),
            AnyValue::String("muffins recipe".into())
        );
    }

    #[test]
    fn test_dynamic_try_into_string() {
        let res: Result<String, _> = AnyValue::String("muffins recipe".into()).try_into();
        assert_eq!(res, Ok("muffins recipe".to_owned()));
        let res: Result<String, _> = AnyValue::Int32(321).try_into();
        assert_eq!(res, Err(TryFromValueError));
    }

    #[test]
    fn test_dynamic_from_str() {
        assert_eq!(
            AnyValue::from("cookies recipe"),
            AnyValue::String("cookies recipe".into())
        );
    }

    #[test]
    fn test_dynamic_try_into_str() {
        let value = AnyValue::String("muffins recipe".into());
        let res: Result<&str, _> = (&value).try_into();
        assert_eq!(res, Ok("muffins recipe"));
        let res: Result<&str, _> = (&AnyValue::Int32(321)).try_into();
        assert_eq!(res, Err(TryFromValueError));
    }

    #[test]
    fn test_dynamic_as_string() {
        assert_eq!(
            AnyValue::from("muffins").as_string(),
            Some(&"muffins".to_owned())
        );
        assert_eq!(AnyValue::Int32(321).as_string(), None);
    }

    #[test]
    fn test_dynamic_as_str() {
        assert_eq!(AnyValue::from("cupcakes").as_str(), Some("cupcakes"));
        assert_eq!(AnyValue::Float(3.14).as_str(), None);
    }

    #[test]
    fn test_dynamic_from_tuple() {
        assert_eq!(
            AnyValue::from(Tuple::default()),
            AnyValue::Tuple(Tuple {
                name: Default::default(),
                elements: Default::default()
            }),
        );
    }

    #[test]
    fn test_dynamic_try_into_tuple() {
        let t: Result<Tuple, _> = AnyValue::Tuple(Tuple {
            name: Default::default(),
            elements: Default::default(),
        })
        .try_into();
        assert_eq!(t, Ok(Tuple::default()));
        let t: Result<Tuple, _> = AnyValue::from("cheesecake").try_into();
        assert_eq!(t, Err(TryFromValueError));
    }

    #[test]
    fn test_dynamic_as_tuple() {
        assert_eq!(
            AnyValue::Tuple(Default::default()).as_tuple(),
            Some(&Tuple::default())
        );
        assert_eq!(AnyValue::Int32(42).as_tuple(), None);
    }

    #[test]
    fn test_dynamic_as_tuple_mut() {
        assert_eq!(
            AnyValue::Tuple(Default::default()).as_tuple_mut(),
            Some(&mut Tuple::default())
        );
        assert_eq!(AnyValue::Int32(42).as_tuple_mut(), None);
    }

    #[test]
    fn test_to_dynamic_value() {
        use crate::typesystem::Value;
        let (s, expected) = sample_serializable_and_dynamic_value();
        let value = to_any_value(&s, Serializable::get_type()).unwrap();
        assert_eq!(value, expected);
    }

    #[test]
    fn test_dynamic_to_dynamic_value() {
        use crate::typesystem::Value;
        let (_, src_value) = sample_serializable_and_dynamic_value();
        let value = to_any_value(&src_value, Serializable::get_type()).unwrap();
        assert_eq!(value, src_value);
    }

    #[test]
    fn test_from_dynamic_value() {
        let (expected, v) = sample_serializable_and_dynamic_value();
        let s: Serializable = from_any_value(&v).unwrap();
        assert_eq!(s, expected);
    }

    #[test]
    fn test_dynamic_from_dynamic_value() {
        let (_, src_value) = sample_serializable_and_dynamic_value();
        let value: AnyValue = from_any_value(&src_value).unwrap();
        assert_eq!(value, src_value);
    }

    #[test]
    fn test_to_from_dynamic_value_invariant() {
        use crate::typesystem::Value;
        let (s, _) = crate::tests::sample_serializable_and_dynamic_value();
        let s2: Serializable =
            from_any_value(&to_any_value(&s, Serializable::get_type()).unwrap()).unwrap();
        assert_eq!(s, s2);
        Ok(())
    }

    #[test]
    fn test_dynamic_ser_de() {
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &AnyValue::Tuple(Tuple::new(tuple::Elements::from_iter([
                AnyValue::List {
                    value_type: Type::String,
                    list: vec![
                        AnyValue::String("cookies".into()),
                        AnyValue::String("muffins".into()),
                    ],
                },
                AnyValue::Int32(12),
                AnyValue::Option {
                    value_type: Type::map(Type::String, Type::Float),
                    option: None,
                },
            ]))),
            &[
                Token::Tuple { len: 2 },
                Token::Str("([s]i+{sf})"),
                Token::Tuple { len: 3 },
                Token::Seq { len: Some(2) },
                Token::Str("cookies"),
                Token::Str("muffins"),
                Token::SeqEnd,
                Token::I32(12),
                Token::None,
                Token::TupleEnd,
            ],
        )
    }
}
