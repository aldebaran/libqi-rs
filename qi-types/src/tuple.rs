use crate::{ty, Type, Value};
use derive_more::{AsRef, From, Index, Into, IntoIterator};

impl ty::StaticGetType for () {
    fn ty() -> Type {
        Type::Unit
    }
}

/// [`Tuple`] represents a `tuple` value in the `qi` type system.
#[derive(Default, Clone, PartialEq, Eq, From, Into, Index, IntoIterator, AsRef, Debug)]
#[into_iterator(owned, ref)]
pub struct Tuple(Vec<Value>);

impl Tuple {
    pub fn new() -> Self {
        Self::unit()
    }

    pub fn from_vec(v: Vec<Value>) -> Self {
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

    pub fn elements(&self) -> &Vec<Value> {
        &self.0
    }
}

impl std::fmt::Display for Tuple {
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

impl ty::DynamicGetType for Tuple {
    fn ty(&self) -> Option<Type> {
        Some(Type::Tuple(ty::TupleType::Tuple(
            self.0.iter().map(|element| element.ty()).collect(),
        )))
    }

    fn current_ty(&self) -> Type {
        Type::Tuple(ty::TupleType::Tuple(
            self.0
                .iter()
                .map(|element| Some(element.current_ty()))
                .collect(),
        ))
    }
}

impl serde::Serialize for Tuple {
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

// Tuples size is not known at compile time, which means we cannot provide it as information to
// serde when deserializing a new tuple. We must rely on what the deserializer knows of the
// value and information it can provide us.
impl<'de> serde::Deserialize<'de> for Tuple {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Tuple;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple value")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Tuple::from_vec(vec![]))
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = serde::Deserialize::deserialize(deserializer)?;
                Ok(Tuple::from_vec(vec![value]))
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
                Ok(Tuple::from_vec(elements))
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
                    let element = Value::Tuple(Tuple::from_vec(vec![key, value]));
                    elements.push(element);
                }
                Ok(Tuple::from_vec(elements))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

#[macro_export]
macro_rules! tuple {
    ($($t:expr),+ $(,)*) => {
        $crate::tuple::Tuple::from_elements(
            vec![$($t),+]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_de_tokens, assert_tokens, Token};

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
                Value::from(Tuple::from_vec(vec![
                    Value::from("thirty two point five"),
                    Value::from(32.5f32),
                ])),
                Value::from(Tuple::from_vec(vec![
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
