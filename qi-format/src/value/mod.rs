pub mod de;
pub use de::*;
pub mod ser;
pub use ser::*;

use crate::{Bool, Number, Option, Raw, String, Tuple, Type};
use derive_more::{From, Index, IndexMut, Into, IntoIterator, TryInto};
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, iter::FromIterator};

/// The [`Value`] structure represents the `value` type in the `qi` format and
/// is is an enumeration of every types of values defined in the format.
///
/// In this type, `unit` values are represented as a `tuple` of size 0.
///
/// # Serialization
///
/// Any serializable value can be represented as a `Value`. They can be both serialized and
/// deserialized to and from `Value`s.
// TODO: insert example here
///
/// `Value`s are serialized transparently. This means that, for instance, a `Value::String(s)` is
/// serialized as would the string `s` be.
// TODO: insert example here
///
/// `Value`s cannot be deserialized from tuples in the `qi` format directly, because the
/// `qi` format is not self-describing, and `value` deserialization is therefore ambiguous.
///
/// This is what the [`AnnotatedValue`] type is for. If you want to deserialize a value from the
/// format, deserialize an [`AnnotatedValue`] instead and then convert it into a `Value`.
// TODO: insert example here
///
/// `Value`s may however be deserialized from a self-describing format.
// TODO: insert example here
// TODO: Implement PartialOrd manually.
#[derive(Clone, From, TryInto, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[try_into(owned, ref, ref_mut)]
pub enum Value<'v> {
    Bool(Bool),
    #[from(ignore)]
    Number(Number),
    String(String<'v>),
    Raw(Raw<'v>),
    #[try_into(ignore)] // Conflicts with standard conversion T -> Opt<T>
    Option(Box<Option<'v>>),
    List(List<'v>),
    Map(Map<'v>),
    Tuple(Tuple<'v>),
}

impl<'v> Value<'v> {
    pub fn unit() -> Self {
        Self::Tuple(Tuple::unit())
    }

    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool(_) => Type::Bool,
            Value::Number(n) => n.get_type(),
            Value::String(_) => Type::String,
            Value::Raw(_) => Type::Raw,
            Value::Option(option) => Type::Option(Box::new(match option.as_ref() {
                Some(value) => value.get_type(),
                None => Type::Dynamic,
            })),
            Value::List(list) => Type::List(Box::new(iter_common_value_type(list))),
            Value::Map(map) => Type::Map {
                key: Box::new(iter_common_value_type(map.keys())),
                value: Box::new(iter_common_value_type(map.values())),
            },
            Value::Tuple(t) => Type::Tuple {
                elements: t.into_iter().map(Self::get_type).collect(),
                annotations: None,
            },
        }
    }

    // TODO: as_XXX functions

    pub fn as_string(&self) -> std::option::Option<&String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> std::option::Option<&Tuple> {
        match self {
            Self::Tuple(t) => Some(t),
            _ => None,
        }
    }
}

impl<'v> Default for Value<'v> {
    fn default() -> Self {
        Self::unit()
    }
}

impl<'v, T> From<T> for Value<'v>
where
    Number: From<T>,
{
    fn from(v: T) -> Self {
        Value::Number(Number::from(v))
    }
}

impl<'v> From<&'v str> for Value<'v> {
    fn from(s: &'v str) -> Self {
        Value::String(String::from(s))
    }
}

impl<'v> From<std::string::String> for Value<'v> {
    fn from(s: std::string::String) -> Self {
        Value::String(String::from(s))
    }
}

impl<'v> From<&'v [u8]> for Value<'v> {
    fn from(b: &'v [u8]) -> Self {
        Self::Raw(Raw::from(b))
    }
}

impl<'v> From<Vec<u8>> for Value<'v> {
    fn from(b: Vec<u8>) -> Self {
        Self::Raw(Raw::from(b))
    }
}

impl<'v> From<Option<'v>> for Value<'v> {
    fn from(o: Option<'v>) -> Self {
        Self::Option(Box::new(o))
    }
}

fn iter_common_value_type<'v, I>(iter: I) -> Type
where
    I: IntoIterator<Item = &'v Value<'v>>,
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

pub type List<'v> = Vec<Value<'v>>;

/// The [`Map`] value represents an association of keys to values in the `qi` format. Both keys and
/// values are `Value`s.
///
/// # Unicity and order of keys
///
/// This type does *not* guarantee unicity of keys in the map. This means that if a map value is
/// read from the `qi` format contains multiple equivalent keys, these keys will be duplicated in
/// the resulting `Map` value.
#[derive(
    new,
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    From,
    Into,
    Index,
    IndexMut,
    IntoIterator,
    Hash,
    Debug,
)]
#[into_iterator(owned, ref, ref_mut)]
pub struct Map<'v>(pub(crate) Vec<(Value<'v>, Value<'v>)>);

impl<'v> Map<'v> {
    pub fn keys(&self) -> impl Iterator<Item = &Value> {
        self.0.iter().map(|(k, _v)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.0.iter().map(|(_k, v)| v)
    }
}

impl<'v, V> FromIterator<V> for Map<'v>
where
    Vec<(Value<'v>, Value<'v>)>: FromIterator<V>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = V>,
    {
        Self(iter.into_iter().collect())
    }
}

/// A value annotated with its type signature.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct AnnotatedValue<'v> {
    r#type: Type,
    value: Value<'v>,
}

impl<'v> AnnotatedValue<'v> {
    pub fn new(value: Value<'v>) -> Self {
        Self {
            r#type: value.get_type(),
            value,
        }
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn into_value(self) -> Value<'v> {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::*, *};
    use pretty_assertions::assert_eq;
    use serde_test::{assert_de_tokens, assert_ser_tokens, assert_tokens, Token};

    #[test]
    fn test_value_from_option() {
        todo!()
    }

    #[test]
    fn test_value_as_string() {
        assert_eq!(
            Value::from("muffins").as_string(),
            Some(&String::from("muffins"))
        );
        assert_eq!(Value::from(Number::Int32(321)).as_string(), None);
    }

    #[test]
    fn test_value_as_tuple() {
        assert_eq!(
            Value::from(Tuple::default()).as_tuple(),
            Some(&Tuple::default())
        );
        assert_eq!(Value::from(Number::Int32(42)).as_tuple(), None);
    }

    #[test]
    fn test_to_value() {
        let s = Serializable::sample();
        let expected = Serializable::sample_as_value();
        let value = to_value(&s);
        assert_eq!(value, expected);
    }

    #[test]
    fn test_value_to_value() {
        let src_value = Serializable::sample_as_value();
        let value = to_value(&src_value);
        assert_eq!(value, src_value);
    }

    #[test]
    fn test_from_value() {
        let expected = Serializable::sample();
        let value = Serializable::sample_as_value();
        let s: Serializable = from_value(value).unwrap();
        assert_eq!(s, expected);
    }

    // Type information is lost each time we serialize or deserialize a value because of
    // type-erasure. Here, the deserialized value is not the same as the source value because
    // tuples are by default interpreted as lists.
    #[test]
    fn test_value_from_value() {
        let value = Serializable::sample_as_value();
        let value: Value = from_value(value).unwrap();
        assert_eq!(
            value,
            // Serializable
            Value::List(vec![
                // S0
                Value::List(vec![
                    // t
                    Value::List(vec![
                        Value::from(-8i8),
                        Value::from(8u8),
                        Value::from(-16i16),
                        Value::from(16u16),
                        Value::from(-32i32),
                        Value::from(32u32),
                        Value::from(-64i64),
                        Value::from(64u64),
                        Value::from(32.32f32),
                        Value::from(64.64f64),
                    ]),
                    // r
                    Value::Raw(vec![51, 52, 53, 54].into()),
                    // o
                    Value::from(Some(Value::from(false))),
                    // s: S1
                    Value::List(vec![Value::from("bananas"), Value::from("oranges")]),
                    // l
                    Value::List(vec![Value::from("cookies"), Value::from("muffins")]),
                    // m
                    Value::from(Map::from(vec![
                        (Value::from(1i32), Value::String("hello".into())),
                        (Value::from(2i32), Value::String("world".into())),
                    ]))
                ])
            ])
        );
    }

    #[test]
    fn test_to_from_value_invariant() {
        let src_s = Serializable::sample();
        let s: Serializable = from_value(to_value(&src_s)).unwrap();
        assert_eq!(s, src_s);
    }

    struct SampleTupleValue;

    impl SampleTupleValue {
        fn tokens() -> Vec<Token> {
            vec![
                Token::Tuple { len: 5 },
                Token::Seq { len: Some(2) },
                Token::Str("cookies"),
                Token::Str("muffins"),
                Token::SeqEnd,
                Token::Bytes(&[1, 2, 3, 4]),
                Token::I32(12),
                Token::Some,
                Token::Map { len: Some(2) },
                Token::Str("pi"),
                Token::F32(std::f32::consts::PI),
                Token::Str("tau"),
                Token::F32(std::f32::consts::TAU),
                Token::MapEnd,
                Token::None,
                Token::TupleEnd,
            ]
        }

        /// The value that gets serialized into `tokens()`.
        fn source_value() -> Value<'static> {
            Value::from(Tuple::new(vec![
                Value::from(List::from(vec![
                    Value::String("cookies".into()),
                    Value::String("muffins".into()),
                ])),
                Value::from(Raw::from(vec![1, 2, 3, 4])),
                Value::from(12i32),
                Value::from(Some(
                    Value::Map(Map(vec![
                        (Value::from("pi"), Value::from(std::f32::consts::PI)),
                        (Value::from("tau"), Value::from(std::f32::consts::TAU)),
                    ]))
                    .into(),
                )),
                Value::from(None),
            ]))
        }

        /// The value that gets deserialized from `tokens()`.
        fn deserialized_value() -> Value<'static> {
            Value::from(List::from(vec![
                Value::from(List::from(vec![
                    Value::String("cookies".into()),
                    Value::String("muffins".into()),
                ])),
                Value::from(Raw::from(vec![1, 2, 3, 4])),
                Value::from(12i32),
                Value::from(Some(
                    Value::from(Map::from(vec![
                        (Value::from("pi"), Value::from(std::f32::consts::PI)),
                        (Value::from("tau"), Value::from(std::f32::consts::TAU)),
                    ]))
                    .into(),
                )),
                Value::from(None),
            ]))
        }

        fn signature() -> &'static str {
            "([s]ri+{sf}+l)<S,l,r,i,om,ol>"
        }
    }

    #[test]
    fn test_value_ser_de() {
        assert_ser_tokens(
            &SampleTupleValue::source_value(),
            &SampleTupleValue::tokens(),
        );
        assert_de_tokens(
            &SampleTupleValue::deserialized_value(),
            &SampleTupleValue::tokens(),
        );
    }

    #[test]
    fn test_map_ser_de() {
        assert_tokens(
            &Map(vec![
                (Value::from(32i16), Value::from("trente deux")),
                (Value::from(34i16), Value::from("trente quatre")),
            ]),
            &[
                Token::Map { len: Some(2) },
                Token::I16(32),
                Token::BorrowedStr("trente deux"),
                Token::I16(34),
                Token::BorrowedStr("trente quatre"),
                Token::MapEnd,
            ],
        );
    }

    // Serialization is symmetric because `AnnotatedValue` carries type information.
    #[test]
    fn test_annotated_value_ser_de() {
        let value = SampleTupleValue::source_value();
        assert_tokens(&AnnotatedValue::new(value), &{
            let mut tokens = vec![
                Token::Tuple { len: 2 },
                Token::Str(SampleTupleValue::signature()),
            ];
            tokens.extend(SampleTupleValue::tokens());
            tokens.push(Token::TupleEnd);
            tokens
        });
    }
}
