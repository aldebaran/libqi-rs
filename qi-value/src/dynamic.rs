mod de;

pub use de::deserialize;
use qi_type::Typed;

pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Typed + serde::Serialize,
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;
    let sig = T::signature();
    let mut struct_serializer = serializer.serialize_struct("Dynamic", 2)?;
    struct_serializer.serialize_field("signature", &sig)?;
    struct_serializer.serialize_field("value", &value)?;
    struct_serializer.end()
}

#[derive(
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    derive_new::new,
    derive_more::From,
)]
pub struct Dynamic<T>(pub T);

impl<T> serde::Serialize for Dynamic<T>
where
    T: Typed + serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self::serialize(&self.0, serializer)
    }
}

impl<'de, T> serde::Deserialize<'de> for Dynamic<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = de::deserialize(deserializer)?;
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_test::{assert_tokens, Token};
    use std::collections::BTreeMap;

    #[test]
    fn test_dynamic_serde_struct() {
        #[derive(PartialEq, Debug, qi_derive::Typed, serde::Serialize, serde::Deserialize)]
        struct MyStruct {
            an_int: i32,
            a_raw: Bytes,
            an_option: Option<BTreeMap<String, Vec<bool>>>,
        }
        assert_tokens(
            &Dynamic(MyStruct {
                an_int: 42,
                a_raw: Bytes::from_static(&[1, 2, 3]),
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
                Token::Struct {
                    name: "MyStruct",
                    len: 3,
                },
                Token::Str("an_int"),
                Token::I32(42),
                Token::Str("a_raw"),
                Token::Bytes(&[1, 2, 3]),
                Token::Str("an_option"),
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
                Token::StructEnd,
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn test_dynamic_serde_with() {
        #[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        struct DynString<'a>(#[serde(with = "super")] &'a str);
        assert_tokens(
            &DynString("Cookies are good"),
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
