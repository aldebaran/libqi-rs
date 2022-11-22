pub mod de;
pub mod ser;
use crate::{Signature, Type};
pub use de::{from_borrowed_value, from_value};
pub use ser::to_value;

use derive_more::{From, Index, IndexMut, Into, IntoIterator, TryInto};
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, option::Option as StdOption, string::String as StdString};

/// The [`Value`] structure represents the sum-type of any value in `qi` format.
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
/// `Value`s cannot be deserialized from tuples in the `qi` format directly. This is because the
/// `qi` format is not self-describing, and the type of values can therefore not be deduced at
/// runtime without additional type information. This is what the [`AnnotatedValue`] type is for.
/// If you want to deserialize a value from the format, deserialize an [`AnnotatedValue`] instead
/// and then convert it into a `Value`.
// TODO: insert example here
///
/// `Value`s may however be deserialized from any self-describing format.
// TODO: insert example here
// TODO: Implement PartialOrd manually.
#[derive(Default, Clone, PartialEq, From, TryInto, Debug)]
#[try_into(owned, ref, ref_mut)]
pub enum Value<'v> {
    #[default]
    Unit,
    Bool(Bool),
    Int8(Int8),
    UnsignedInt8(UnsignedInt8),
    Int16(Int16),
    UnsignedInt16(UnsignedInt16),
    Int32(Int32),
    UnsignedInt32(UnsignedInt32),
    Int64(Int64),
    UnsignedInt64(UnsignedInt64),
    Float32(Float32),
    Float64(Float64),
    String(String<'v>),
    Raw(Raw<'v>),
    Option(Option<'v>),
    List(List<'v>),
    Map(Map<'v>),
    Tuple(Tuple<'v>),
    // TODO: Handle enumerations
}

impl<'v> Value<'v> {
    pub fn get_type(&self) -> Type {
        match self {
            Value::Unit => Type::Void,
            Value::Bool(_) => Type::Bool,
            Value::Int8(_) => Type::Int8,
            Value::UnsignedInt8(_) => Type::UInt8,
            Value::Int16(_) => Type::UInt16,
            Value::UnsignedInt16(_) => Type::UInt16,
            Value::Int32(_) => Type::Int32,
            Value::UnsignedInt32(_) => Type::UInt32,
            Value::Int64(_) => Type::UInt32,
            Value::UnsignedInt64(_) => Type::UInt64,
            Value::Float32(_) => Type::Float,
            Value::Float64(_) => Type::Double,
            Value::String(_) => Type::String,
            Value::Raw(_) => Type::Raw,
            Value::Option(option) => Type::Option(Box::new(match option {
                Some(value) => value.get_type(),
                None => Type::Dynamic,
            })),
            Value::List(list) => Type::List(Box::new(iter_common_value_type(list))),
            Value::Map(map) => Type::Map {
                key: Box::new(iter_common_value_type(map.keys())),
                value: Box::new(iter_common_value_type(map.values())),
            },
            Value::Tuple(Tuple { elements }) => {
                Type::Tuple(elements.iter().map(Self::get_type).collect())
            }
        }
    }

    // TODO: as_XXX functions

    pub fn as_string(&self) -> StdOption<&String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_std_string(&self) -> StdOption<StdString> {
        match self {
            Self::String(s) => Some(s.clone().into_owned()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> StdOption<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> StdOption<&Tuple> {
        match self {
            Self::Tuple(t) => Some(t),
            _ => None,
        }
    }
}

impl<'s> From<&'s str> for Value<'s> {
    fn from(s: &'s str) -> Self {
        Self::String(String::Borrowed(s))
    }
}

impl<'v> From<StdString> for Value<'v> {
    fn from(s: StdString) -> Self {
        Self::String(String::Owned(s))
    }
}

impl<'v> std::convert::TryFrom<&'v Value<'v>> for &'v str {
    type Error = &'static str;
    fn try_from(value: &'v Value) -> Result<Self, Self::Error> {
        let str: &String = value.try_into()?;
        Ok(str)
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

pub type Bool = bool;
pub type Int8 = i8;
pub type UnsignedInt8 = u8;
pub type Int16 = i16;
pub type UnsignedInt16 = u16;
pub type Int32 = i32;
pub type UnsignedInt32 = u32;
pub type Int64 = i64;
pub type UnsignedInt64 = u64;
pub type Float32 = f32;
pub type Float64 = f64;
pub type String<'s> = Cow<'s, str>;
pub type Raw<'r> = Cow<'r, [u8]>;
pub type Option<'v> = StdOption<Box<Value<'v>>>;
pub type List<'v> = Vec<Value<'v>>;

/// The [`Map`] value represents an association of keys to values in the `qi` format. Both keys and
/// values are `Value`s.
///
/// # Unicity and order of keys
///
/// This type does *not* guarantee unicity of keys in the map. This means that if a map value is
/// read from the `qi` format contains multiple equivalent keys, these keys will be duplicated in
/// the resulting `Map` value.
#[derive(new, Default, Clone, PartialEq, From, Into, Index, IndexMut, IntoIterator, Debug)]
#[into_iterator(owned, ref, ref_mut)]
pub struct Map<'v>(Vec<(Value<'v>, Value<'v>)>);

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

/// The [`Tuple`] value represents a tuple of values in the `qi` format.
///
/// # Serialization
///
/// [`Tuple`] values cannot be deserialized from tuples in the `qi` format directly. This is
/// because tuple deserialization requires the length of the tuple to be known at compile time,
/// which we cannot determine from the format values without additional type information.
///
/// If you need to deserialize a tuple and you know its size, try deserializing a builtin tuple
/// of values instead.
///
/// ```
/// # use qi_serde::{Error, value::*};
/// use qi_serde::from_bytes;
///
/// # fn main() -> Result<(), Error> {
/// let bytes = [1, 0, 2, 0, 3, 0];
/// let values : (i16, i16, i16) = from_bytes(&bytes)?;
/// assert_eq!(values, (1, 2, 3));
/// # Ok(())
/// # }
/// ```
///
/// You can however deserialize a tuple out of an annotated value.
///
/// ```
/// # use qi_serde::{Error, value::*};
/// use qi_serde::from_bytes;
///
/// # fn main() -> Result<(), Error> {
/// let bytes = [3, 0, 0, 0, 40, 105, 105, 41, 10, 20];
/// let annotated_value : AnnotatedValue = from_bytes(&bytes)?;
/// let value = annotated_value.into_value();
/// assert_eq!(value.as_tuple(),
///            Some(&Tuple::new(vec![
///                Value::Int32(10),
///                Value::Int32(20)
///            ])));
/// # Ok(())
/// # }
/// ```
/// Tuples may also be deserialized from:
///   - sequences, as tuples of the sequences elements.
///   - maps, as tuples of pairs (tuples of length 2).
///   - unit values, as tuples of length 0.
///   - newtype structures, as tuples of length 1.
#[derive(new, Default, Clone, PartialEq, From, Into, Index, IndexMut, IntoIterator, Debug)]
#[into_iterator(owned, ref, ref_mut)]
pub struct Tuple<'v> {
    elements: Vec<Value<'v>>,
}

impl<'v> Tuple<'v> {
    pub fn elements(&self) -> &Vec<Value<'v>> {
        &self.elements
    }
}

/// An value annotated with its type signature in the `qi` format.
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct AnnotatedValue<'v> {
    signature: Signature,
    value: Value<'v>,
}

impl<'v> AnnotatedValue<'v> {
    pub fn new(value: Value<'v>) -> Self {
        Self {
            signature: Signature::new(value.get_type()),
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

impl<'v> From<Value<'v>> for AnnotatedValue<'v> {
    fn from(v: Value<'v>) -> Self {
        Self::new(v)
    }
}

impl<'v> From<AnnotatedValue<'v>> for Value<'v> {
    fn from(v: AnnotatedValue<'v>) -> Self {
        v.value
    }
}

impl<'v> From<&'v AnnotatedValue<'v>> for &'v Value<'v> {
    fn from(v: &'v AnnotatedValue<'v>) -> Self {
        &v.value
    }
}

impl<'v> From<&'v mut AnnotatedValue<'v>> for &'v mut Value<'v> {
    fn from(v: &'v mut AnnotatedValue<'v>) -> Self {
        &mut v.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use assert_matches::assert_matches;
    use pretty_assertions::assert_eq;
    use serde_test::{assert_de_tokens, assert_tokens, Token};

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
        assert_matches!(res, Ok(str) => str == "muffins recipe");
        let res: Result<String, _> = Value::Int32(321).try_into();
        assert_matches!(res, Err(""));
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
        assert_matches!(res, Err(err) => err.contains("Only String can be converted"));
    }

    #[test]
    fn test_value_as_string() {
        assert_eq!(
            Value::from("muffins").as_string(),
            Some(&Cow::Borrowed("muffins"))
        );
        assert_eq!(Value::Int32(321).as_string(), None);
    }

    #[test]
    fn test_value_as_std_string() {
        assert_eq!(
            Value::from("cheesecake").as_std_string(),
            Some("cheesecake".to_owned())
        );
        assert_eq!(Value::Int32(321).as_std_string(), None);
    }

    #[test]
    fn test_value_as_str() {
        assert_eq!(Value::from("cupcakes").as_str(), Some("cupcakes"));
        assert_eq!(Value::Float32(3.15).as_str(), None);
    }

    #[test]
    fn test_value_as_tuple() {
        assert_eq!(
            Value::Tuple(Tuple::default()).as_tuple(),
            Some(&Tuple::default())
        );
        assert_eq!(Value::Int32(42).as_tuple(), None);
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
        let s: Serializable = from_borrowed_value(&value).unwrap();
        assert_eq!(s, expected);
    }

    // Type information is lost each time we serialize or deserialize a value because of
    // type-erasure. Here, the deserialized value is not the same as the source value because
    // tuples are by default interpreted as lists.
    #[test]
    fn test_value_from_value() {
        let value = Serializable::sample_as_value();
        let value: Value = from_borrowed_value(&value).unwrap();
        assert_eq!(
            value,
            // Serializable
            Value::List(vec![
                // S0
                Value::List(vec![
                    // t
                    Value::List(vec![
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
                    ]),
                    // r
                    Value::Raw(vec![51, 52, 53, 54].into()),
                    // o
                    Value::Option(Some(Value::Bool(false).into())),
                    // s: S1
                    Value::List(vec![Value::from("bananas"), Value::from("oranges")]),
                    // l
                    Value::List(vec![Value::from("cookies"), Value::from("muffins")]),
                    // m
                    Value::from(Map::from(vec![
                        (Value::Int32(1), Value::String("hello".into())),
                        (Value::Int32(2), Value::String("world".into())),
                    ]))
                ])
            ])
        );
    }

    #[test]
    fn test_to_from_value_invariant() {
        let src_s = Serializable::sample();
        let s: Serializable = from_borrowed_value(&to_value(&src_s)).unwrap();
        assert_eq!(s, src_s);
    }

    struct SampleTupleValue {
        value: Value<'static>,
        tokens: Vec<Token>,
        signature: &'static str,
    }
    impl SampleTupleValue {
        fn new() -> Self {
            Self {
                value: Value::Tuple(Tuple {
                    elements: vec![
                        Value::List(vec![
                            Value::String("cookies".into()),
                            Value::String("muffins".into()),
                        ]),
                        Value::Raw(Raw::from(vec![1, 2, 3, 4])),
                        Value::Int32(12),
                        Value::Option(Some(
                            Value::Map(Map(vec![
                                (
                                    Value::String("pi".into()),
                                    Value::Float32(std::f32::consts::PI),
                                ),
                                (
                                    Value::String("tau".into()),
                                    Value::Float32(std::f32::consts::TAU),
                                ),
                            ]))
                            .into(),
                        )),
                        Value::Option(None),
                    ],
                }),
                tokens: vec![
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
                ],
                signature: "([s]ri+{sf}+l)<S,l,r,i,om,ol>",
            }
        }
    }

    #[test]
    fn test_value_ser_de() {
        let sample_value = SampleTupleValue::new();
        assert_tokens(&sample_value.value, &sample_value.tokens)
    }

    #[test]
    fn test_map_ser_de() {
        assert_tokens(
            &Map(vec![
                (Value::Int16(32), Value::String("trente deux".into())),
                (Value::Int16(34), Value::String("trente quatre".into())),
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

    #[test]
    fn test_tuple_ser_de() {
        assert_tokens(
            &Tuple {
                elements: vec![Value::Int16(32), Value::Int32(34), Value::Float64(132.29)],
            },
            &[
                Token::Tuple { len: 3 },
                Token::I16(32),
                Token::I32(34),
                Token::F64(132.29),
                Token::TupleEnd,
            ],
        );
    }

    // Tuples can be deserialized from sequences, even when size is unknown.
    #[test]
    fn test_tuple_de_seq() {
        assert_de_tokens(
            &Tuple {
                elements: vec![Value::Int32(42), Value::String("cookies".into())],
            },
            &[
                Token::Seq { len: None },
                Token::I32(42),
                Token::BorrowedStr("cookies"),
                Token::SeqEnd,
            ],
        )
    }

    // Tuples can be deserialized from maps, even when size is unknown.
    #[test]
    fn test_tuple_de_maps() {
        assert_de_tokens(
            &Tuple {
                elements: vec![
                    Value::Tuple(Tuple {
                        elements: vec![
                            Value::String("thirty two point five".into()),
                            Value::Float32(32.5),
                        ],
                    }),
                    Value::Tuple(Tuple {
                        elements: vec![
                            Value::String("thirteen point three".into()),
                            Value::Float32(13.3),
                        ],
                    }),
                ],
            },
            &[
                Token::Map { len: None },
                Token::BorrowedStr("thirty two point five"),
                Token::F32(32.5),
                Token::BorrowedStr("thirteen point three"),
                Token::F32(13.3),
                Token::MapEnd,
            ],
        )
    }

    // Tuples can be deserialized from unit values.
    #[test]
    fn test_tuple_de_unit() {
        assert_de_tokens(&Tuple { elements: vec![] }, &[Token::Unit])
    }

    // Tuples can be deserialized from newtype struct values.
    #[test]
    fn test_tuple_de_newtype() {
        assert_de_tokens(
            &Tuple {
                elements: vec![Value::Int8(64)],
            },
            &[Token::NewtypeStruct { name: "MyStruct" }, Token::I8(64)],
        )
    }

    #[test]
    fn test_annotated_value_ser_de() {
        let sample_value = SampleTupleValue::new();
        assert_tokens(&AnnotatedValue::new(sample_value.value), &{
            let mut tokens = vec![Token::Tuple { len: 2 }, Token::Str(sample_value.signature)];
            tokens.extend(sample_value.tokens);
            tokens.push(Token::TupleEnd);
            tokens
        });
    }
}
