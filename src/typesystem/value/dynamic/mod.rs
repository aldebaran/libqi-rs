mod de;
mod ser;
use super::super::Type;
pub use de::from_value;
pub use ser::to_value;

pub struct Dynamic {
    r#type: Type,
    value: Value,
}

// TODO: #[non_exhaustive]
// TODO: Enable the value to borrow data from sources.
// TODO: This is a dynamic value, and should be de/serialized as such.
#[derive(Default, Clone, PartialEq, PartialOrd, Debug)]
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
    Optional(Option<Box<Value>>),
    List(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Tuple(Tuple),
    // TODO: Handle enumerations
}

pub mod tuple {
    use super::Value;
    use crate::typesystem::tuple;
    pub type Tuple = tuple::Tuple<Value>;
    pub type Elements = tuple::Elements<Value>;
    pub type Field = tuple::Field<Value>;
}
pub use tuple::Tuple;

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

impl From<Tuple> for Value {
    fn from(t: Tuple) -> Self {
        Value::Tuple(t)
    }
}

impl TryFrom<Value> for Tuple {
    type Error = TryFromValueError;

    fn try_from(d: Value) -> Result<Self, Self::Error> {
        match d {
            Value::Tuple(t) => Ok(t),
            _ => Err(TryFromValueError),
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[error("dynamic value conversion failed")]
pub struct TryFromValueError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
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
        assert_eq!(Value::Float(3.14).as_str(), None);
    }

    #[test]
    fn test_value_from_tuple() {
        assert_eq!(
            Value::from(Tuple::default()),
            Value::Tuple(Tuple {
                name: Default::default(),
                elements: Default::default()
            }),
        );
    }

    #[test]
    fn test_value_try_into_tuple() {
        let t: Result<Tuple, _> = Value::Tuple(Tuple {
            name: Default::default(),
            elements: Default::default(),
        })
        .try_into();
        assert_eq!(t, Ok(Tuple::default()));
        let t: Result<Tuple, _> = Value::from("cheesecake").try_into();
        assert_eq!(t, Err(TryFromValueError));
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
        let (s, expected) = sample_serializable_and_dynamic_value();
        let value = to_value(&s).unwrap();
        assert_eq!(value, expected);
    }

    #[test]
    fn test_from_value() {
        let (expected, v) = sample_serializable_and_dynamic_value();
        let s: Serializable = from_value(v).unwrap();
        assert_eq!(s, expected);
    }

    #[test]
    fn test_to_from_value_invariant() -> Result<(), ser::Error> {
        let (s, _) = crate::tests::sample_serializable_and_dynamic_value();
        let s2: Serializable = from_value(to_value(&s)?)?;
        assert_eq!(s, s2);
        Ok(())
    }

    #[test]
    fn test_value_ser_de() {
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &Value::List(vec![
                Value::String("cookies".into()),
                Value::String("muffins".into()),
            ]),
            &[
                Token::Tuple { len: 2 },
                Token::Str("[s]"),
                Token::Seq { len: Some(2) },
                Token::Str("cookies"),
                Token::Str("muffins"),
                Token::SeqEnd,
            ],
        )
    }
}
