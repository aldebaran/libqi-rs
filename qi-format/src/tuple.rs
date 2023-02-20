use crate::Value;
use derive_more::{AsRef, From, Index, Into, IntoIterator};

/// # Serialization / Deserialization
///
/// This is represented as a `unit` in the Serde data model.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Unit;

impl serde::Serialize for Unit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_unit()
    }
}

impl<'de> serde::Deserialize<'de> for Unit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <()>::deserialize(deserializer).map(|()| Self)
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("()")
    }
}

/// [`Tuple`] represents a `tuple` value in the `qi` type system.
///
/// # Deserialization ambiguity
///
/// Deserializing tuples requires knowing their length. Furthermore, `value`s’ deserialization is
/// ambiguous. This means `tuple`s’ is as well and requires context.
///
/// If you need to deserialize a tuple from the format and you know its length and value types, try
/// deserializing a builtin tuple instead.
///
/// ```
/// # use qi_format::{from_bytes, Result};
/// # fn main() -> Result<()> {
/// let bytes = [1, 0, 2, 0, 3, 0];
/// let values : (i16, i16, i16) = from_bytes(&bytes)?;
/// assert_eq!(values, (1, 2, 3));
/// # Ok(())
/// # }
/// ```
///
/// You can however deserialize a tuple out of a dynamic value.
///
/// ```
/// # use qi_format::{from_bytes, Dynamic, Number, Value, Result, Tuple};
/// # fn main() -> Result<()> {
/// let bytes = [4, 0, 0, 0, 40, 105, 105, 41, 10, 0, 0, 0, 20, 0, 0, 0];
/// let dynamic : Dynamic = from_bytes(&bytes)?;
/// let value = dynamic.into_value();
/// assert_eq!(value.as_tuple(),
///            Some(&Tuple::from_elements(vec![
///                Value::Number(Number::Int32(10)),
///                Value::Number(Number::Int32(20))
///            ])));
/// # Ok(())
/// # }
/// ```
///
/// Tuples may also be deserialized from:
///   - sequences, as tuples of the sequences elements.
///   - maps, as tuples of pairs (tuples of length 2).
///   - unit values, as tuples of length 0.
///   - newtype structures, as tuples of length 1.
#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    From,
    Into,
    Index,
    IntoIterator,
    AsRef,
    Hash,
    Debug,
)]
#[into_iterator(owned, ref)]
pub struct Tuple<'v>(Vec<Value<'v>>);

impl<'v> Tuple<'v> {
    pub fn new() -> Self {
        Self::unit()
    }

    pub fn from_elements(v: Vec<Value<'v>>) -> Self {
        Self(v)
    }

    pub fn unit() -> Self {
        Self(vec![])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn is_unit(&self) -> bool {
        self.0.is_empty()
    }

    pub fn elements(&self) -> &Vec<Value<'v>> {
        &self.0
    }
}

impl<'v> std::fmt::Display for Tuple<'v> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        let mut add_sep = false;
        for element in &self.0 {
            if add_sep {
                f.write_str(", ")?;
            }
            write!(f, "{element}")?;
            add_sep = true;
        }
        f.write_str(")")
    }
}

impl<'v> serde::Serialize for Tuple<'v> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut serializer = serializer.serialize_tuple(self.0.len())?;
        for element in &self.0 {
            serializer.serialize_element(element)?;
        }
        serializer.end()
    }
}

impl<'de> serde::Deserialize<'de> for Tuple<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Tuple<'de>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple value")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Tuple::from_elements(vec![]))
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = serde::Deserialize::deserialize(deserializer)?;
                Ok(Tuple::from_elements(vec![value]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut elements = match seq.size_hint() {
                    Some(size) => Vec::with_capacity(size),
                    None => Vec::new(),
                };
                while let Some(element) = seq.next_element()? {
                    elements.push(element);
                }
                Ok(Tuple::from_elements(elements))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut elements = match map.size_hint() {
                    Some(size) => Vec::with_capacity(size),
                    None => Vec::new(),
                };
                while let Some((key, value)) = map.next_entry()? {
                    let element = Value::Tuple(Tuple::from_elements(vec![key, value]));
                    elements.push(element);
                }
                Ok(Tuple::from_elements(elements))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_de_tokens, assert_tokens, Token};

    #[test]
    fn test_unit_serde() {
        assert_tokens(&Unit, &[Token::Unit]);
    }

    #[test]
    fn test_tuple_serde() {
        assert_tokens(
            &Tuple(vec![
                Value::from(32i16),
                Value::from(34i32),
                Value::from(132.29f64),
            ]),
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
            &Tuple(vec![Value::from(42i32), Value::from("cookies")]),
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
            &Tuple(vec![
                Value::from(Tuple::from_elements(vec![
                    Value::from("thirty two point five"),
                    Value::from(32.5f32),
                ])),
                Value::from(Tuple::from_elements(vec![
                    Value::from("thirteen point three"),
                    Value::from(13.3f32),
                ])),
            ]),
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
        assert_de_tokens(&Tuple::unit(), &[Token::Unit])
    }

    // Tuples can be deserialized from newtype struct values.
    #[test]
    fn test_tuple_de_newtype() {
        assert_de_tokens(
            &Tuple(vec![Value::from(64i8)]),
            &[Token::NewtypeStruct { name: "MyStruct" }, Token::I8(64)],
        )
    }
}
