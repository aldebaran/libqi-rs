mod de;

use crate::{reflect::RuntimeReflect, FromValue, IntoValue, Reflect, ToValue, Value};

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, derive_more::From)]
pub struct Dynamic<T>(pub T);

impl<'a> Dynamic<Value<'a>> {
    pub fn into_owned(self) -> Dynamic<Value<'static>> {
        Dynamic(self.0.into_owned())
    }
}

impl<T> std::fmt::Display for Dynamic<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Reflect for Dynamic<T> {
    fn ty() -> Option<crate::Type> {
        None
    }
}

impl<T> ToValue for Dynamic<T>
where
    T: ToValue,
{
    fn to_value(&self) -> Value<'_> {
        Value::Dynamic(Box::new(self.0.to_value()))
    }
}

impl<'a, T> IntoValue<'a> for Dynamic<T>
where
    T: IntoValue<'a>,
{
    fn into_value(self) -> Value<'a> {
        Value::Dynamic(Box::new(self.0.into_value()))
    }
}

impl<'a, T> FromValue<'a> for Dynamic<T>
where
    T: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, crate::FromValueError> {
        match value {
            Value::Dynamic(v) => Ok(Self(T::from_value(*v)?)),
            _ => Err(crate::FromValueError::TypeMismatch {
                expected: "a dynamic value".to_owned(),
                actual: value.to_string(),
            }),
        }
    }
}

impl<T> serde::Serialize for Dynamic<T>
where
    for<'a> &'a T: IntoValue<'a>,
    T: RuntimeReflect,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self::serialize(&self.0, serializer)
    }
}

impl<'de, 'v, T> serde::Deserialize<'de> for Dynamic<T>
where
    T: FromValue<'v>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self::deserialize(deserializer).map(Self)
    }
}

const SERDE_STRUCT_NAME: &str = "Dynamic";

enum Fields {
    Signature,
    Value,
}

impl Fields {
    const KEYS: [&'static str; 2] = ["signature", "value"];
    const fn key(&self) -> &'static str {
        match self {
            Fields::Signature => Self::KEYS[0],
            Fields::Value => Self::KEYS[1],
        }
    }
}

pub fn serialize<'a, T, S>(value: T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: 'a + IntoValue<'a> + RuntimeReflect,
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;
    let mut serializer = serializer.serialize_struct(SERDE_STRUCT_NAME, Fields::KEYS.len())?;
    serializer.serialize_field(Fields::Signature.key(), &value.signature())?;
    serializer.serialize_field(Fields::Value.key(), &value.into_value())?;
    serializer.end()
}

pub fn deserialize<'de, 'v, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromValue<'v>,
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = deserializer.deserialize_struct(
        SERDE_STRUCT_NAME,
        &Fields::KEYS,
        de::DynamicVisitor::new(),
    )?;
    value
        .cast_into()
        .map_err(|err| D::Error::custom(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};
    use std::collections::BTreeMap;

    #[test]
    fn test_dynamic_serde_struct() {
        #[derive(
            PartialEq, Debug, qi_macros::Reflect, qi_macros::ToValue, qi_macros::FromValue,
        )]
        #[qi(value = "crate")]
        struct MyStruct {
            an_int: i32,
            #[qi(as_raw)]
            a_raw: Vec<u8>,
            an_option: Option<BTreeMap<String, Vec<bool>>>,
        }
        assert_tokens(
            &Dynamic(MyStruct {
                an_int: 42,
                a_raw: vec![1, 2, 3],
                an_option: Some(BTreeMap::from_iter([
                    ("true_true".to_owned(), vec![true, true]),
                    ("false_true".to_owned(), vec![false, true]),
                    ("true_false".to_owned(), vec![true, false]),
                    ("false_false".to_owned(), vec![false, false]),
                ])),
            }),
            &[
                Token::Struct {
                    name: "Dynamic",
                    len: 2,
                },
                Token::Str("signature"),
                Token::Str("(ir+{s[b]})<MyStruct,an_int,a_raw,an_option>"),
                Token::Str("value"),
                Token::Tuple { len: 3 },
                Token::I32(42),
                Token::BorrowedBytes(&[1, 2, 3]),
                Token::Some,
                Token::Map { len: Some(4) },
                Token::Str("false_false"),
                Token::Seq { len: Some(2) },
                Token::Bool(false),
                Token::Bool(false),
                Token::SeqEnd,
                Token::Str("false_true"),
                Token::Seq { len: Some(2) },
                Token::Bool(false),
                Token::Bool(true),
                Token::SeqEnd,
                Token::Str("true_false"),
                Token::Seq { len: Some(2) },
                Token::Bool(true),
                Token::Bool(false),
                Token::SeqEnd,
                Token::Str("true_true"),
                Token::Seq { len: Some(2) },
                Token::Bool(true),
                Token::Bool(true),
                Token::SeqEnd,
                Token::MapEnd,
                Token::TupleEnd,
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn test_dynamic_serde_with() {
        #[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        struct DynString(#[serde(with = "super")] String);
        assert_tokens(
            &DynString("Cookies are good".to_owned()),
            &[
                Token::Struct {
                    name: "Dynamic",
                    len: 2,
                },
                Token::Str("signature"),
                Token::Str("s"),
                Token::Str("value"),
                Token::BorrowedStr("Cookies are good"),
                Token::StructEnd,
            ],
        )
    }
}
