use crate::{
    num_bool::*,
    tuple::*,
    typing::{self, Type},
    List, Map, Object, Option, Raw, Signature, String, Value,
};

/// [`Dynamic`] represents a `dynamic` value in the `qi` type system.
///
/// It is a value associated with its type information.
///
/// It is represented in the format as a value prepended with its type signature.
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct Dynamic {
    // Invariant: `value.is_assignable_to_value_type(&value_type)`
    value_type: Type,
    value: Value,
}

impl Dynamic {
    pub fn from_type_and_value(value_type: Type, value: Value) -> Result<Self, DynamicError> {
        if !value.is_assignable_to_value_type(&value_type) {
            return Err(DynamicError::MismatchedValueType(value, value_type));
        }
        Ok(Self { value_type, value })
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn into_value(self) -> Value {
        self.value
    }

    pub fn is_assignable_to_value_type(&self, t: &Type) -> bool {
        self.value.is_assignable_to_value_type(t)
    }
}

impl std::fmt::Display for Dynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone)]
pub enum DynamicError {
    #[error("mismatched value type: value {0} is not assignable to {1}")]
    MismatchedValueType(Value, Type),
}

impl serde::Serialize for Dynamic {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut serialize_struct = serializer.serialize_struct("qi::Dynamic", 2)?;
        use serde::ser::SerializeStruct;
        serialize_struct.serialize_field(
            DynamicField::SIGNATURE,
            &Signature::new(self.value_type.clone()),
        )?;
        serialize_struct.serialize_field(DynamicField::VALUE, &self.value)?;
        serialize_struct.end()
    }
}

impl<'de> serde::Deserialize<'de> for Dynamic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Dynamic;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a dynamic value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de;

                let invalid_length =
                    |i| de::Error::invalid_length(i, &"a sequence of size 2 (signature, value)");

                // Signature
                let signature: Signature = seq.next_element()?.ok_or_else(|| invalid_length(0))?;
                let value_type = signature.into_type();

                // Value
                let value = seq
                    .next_element_seed(ValueSeed(&value_type))?
                    .ok_or_else(|| invalid_length(1))?;

                Dynamic::from_type_and_value(value_type, value).map_err(|err| {
                    de::Error::custom(format!(
                        "logic error while deserializing a dynamic value, {err}"
                    ))
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                use serde::de;

                // Signature
                let key = map
                    .next_key()?
                    .ok_or_else(|| de::Error::missing_field(DynamicField::SIGNATURE))?;
                let signature: Signature = match key {
                    DynamicField::Value => {
                        return Err(de::Error::missing_field(DynamicField::SIGNATURE))
                    }
                    DynamicField::Signature => map.next_value()?,
                };
                let value_type = signature.into_type();

                // Value
                let key = map
                    .next_key()?
                    .ok_or_else(|| de::Error::missing_field(DynamicField::VALUE))?;
                match key {
                    DynamicField::Signature => {
                        return Err(de::Error::duplicate_field(DynamicField::SIGNATURE))
                    }
                    DynamicField::Value => (),
                }

                let value = map.next_value_seed(ValueSeed(&value_type))?;
                Dynamic::from_type_and_value(value_type, value).map_err(|err| {
                    de::Error::custom(format!(
                        "logic error while deserializing a dynamic value, {err}"
                    ))
                })
            }
        }

        deserializer.deserialize_struct("qi::Dynamic", &DynamicField::NAMES, Visitor)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase", field_identifier)]
enum DynamicField {
    Signature,
    Value,
}

impl DynamicField {
    const SIGNATURE: &'static str = "signature";
    const VALUE: &'static str = "value";
    const NAMES: [&'static str; 2] = [Self::SIGNATURE, Self::VALUE];
}

struct ValueSeed<'t>(&'t Type);

impl<'t, 'de> serde::de::DeserializeSeed<'de> for ValueSeed<'t> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;

        let value = match self.0 {
            Type::Unit => {
                let v = Unit::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Bool => {
                let v = Bool::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Int8 => {
                let v = Int8::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::UInt8 => {
                let v = UInt8::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Int16 => {
                let v = Int16::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::UInt16 => {
                let v = UInt16::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Int32 => {
                let v = Int32::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::UInt32 => {
                let v = UInt32::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Int64 => {
                let v = Int64::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::UInt64 => {
                let v = UInt64::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Float32 => {
                let v = Float32::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Float64 => {
                let v = Float64::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::String => {
                let v = String::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Raw => {
                let v = Raw::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Object => {
                let v = Object::deserialize(deserializer)?;
                Value::from(v)
            }
            Type::Dynamic => {
                let v = Dynamic::deserialize(deserializer)?;
                v.into_value()
            }
            Type::Option(t) => {
                struct Visitor<'t>(&'t Type);
                impl<'t, 'de> serde::de::Visitor<'de> for Visitor<'t> {
                    type Value = Option<Value>;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("an optional value")
                    }

                    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        use serde::de::DeserializeSeed;
                        let value = ValueSeed(self.0).deserialize(deserializer)?;
                        Ok(Some(value))
                    }

                    fn visit_none<E>(self) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        Ok(None)
                    }
                }
                let v = deserializer.deserialize_option(Visitor(t.as_ref()))?;
                Value::from(v)
            }
            Type::List(t) | Type::VarArgs(t) => {
                struct Visitor<'t>(&'t Type);
                impl<'t, 'de> serde::de::Visitor<'de> for Visitor<'t> {
                    type Value = List<Value>;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("a list or varargs value")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>,
                    {
                        let mut list = List::new();
                        while let Some(value) = seq.next_element_seed(ValueSeed(self.0))? {
                            list.push(value);
                        }
                        Ok(list)
                    }
                }
                let v = deserializer.deserialize_seq(Visitor(t.as_ref()))?;
                Value::from(v)
            }
            Type::Map { key, value } => {
                struct Visitor<'t>(&'t Type, &'t Type);
                impl<'t, 'de> serde::de::Visitor<'de> for Visitor<'t> {
                    type Value = Map<Value, Value>;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("a map value")
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        let mut pair_vec = Vec::new();
                        while let Some(kv_pair) =
                            map.next_entry_seed(ValueSeed(self.0), ValueSeed(self.1))?
                        {
                            pair_vec.push(kv_pair);
                        }
                        let map = Map::from_iter(pair_vec);
                        Ok(map)
                    }
                }
                let v = deserializer.deserialize_map(Visitor(key.as_ref(), value.as_ref()))?;
                Value::from(v)
            }
            Type::Tuple(tuple) => {
                struct Visitor<'t>(&'t typing::Tuple);
                impl<'t, 'de> serde::de::Visitor<'de> for Visitor<'t> {
                    type Value = Tuple;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        write!(f, "a tuple value of size {len}", len = self.0.len())
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>,
                    {
                        let mut elements = Vec::new();
                        for element_type in self.0 {
                            match seq.next_element_seed(ValueSeed(element_type))? {
                                Some(element) => elements.push(element),
                                None => {
                                    return Err(serde::de::Error::invalid_length(
                                        elements.len(),
                                        &self,
                                    ))
                                }
                            };
                        }
                        let tuple = Tuple::from_elements(elements);
                        Ok(tuple)
                    }
                }
                let v = deserializer.deserialize_tuple(tuple.len(), Visitor(tuple))?;
                Value::from(v)
            }
        };
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_dynamic_serde() {
        let value_type = Type::Tuple(
            typing::Tuple::from_element_types_with_annotations(
                vec![
                    Type::Int32,
                    Type::Raw,
                    Type::Option(Box::new(Type::Map {
                        key: Box::new(Type::String),
                        value: Box::new(Type::List(Box::new(Type::Bool))),
                    })),
                ],
                typing::TupleAnnotations {
                    name: "MyStruct".to_owned(),
                    fields: Some(vec![
                        "an_int".to_owned(),
                        "a_raw".to_owned(),
                        "an_option".to_owned(),
                    ]),
                },
            )
            .unwrap(),
        );
        let value = Value::Tuple(Tuple::from_elements(vec![
            Value::Number(Number::Int32(42)),
            Value::Raw(Raw::from(vec![1, 2, 3])),
            Value::Option(Box::new(Some(Value::Map(Map::from_iter(vec![
                (
                    Value::String(String::from("true_true")),
                    Value::List(vec![Value::Bool(true), Value::Bool(true)]),
                ),
                (
                    Value::String(String::from("false_true")),
                    Value::List(vec![Value::Bool(false), Value::Bool(true)]),
                ),
                (
                    Value::String(String::from("true_false")),
                    Value::List(vec![Value::Bool(true), Value::Bool(false)]),
                ),
                (
                    Value::String(String::from("false_false")),
                    Value::List(vec![Value::Bool(false), Value::Bool(false)]),
                ),
            ]))))),
        ]));
        let dynamic = Dynamic::from_type_and_value(value_type, value).unwrap();
        assert_tokens(
            &dynamic,
            &[
                Token::Struct {
                    name: "qi::Dynamic",
                    len: 2,
                },
                Token::Str("signature"),
                Token::Str("(ir+{s[b]})<MyStruct,an_int,a_raw,an_option>"),
                Token::Str("value"),
                Token::Tuple { len: 3 },
                Token::I32(42),
                Token::Bytes(&[1, 2, 3]),
                Token::Some,
                Token::Map { len: Some(4) },
                Token::Str("true_true"),
                Token::Seq { len: Some(2) },
                Token::Bool(true),
                Token::Bool(true),
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
                Token::Str("false_false"),
                Token::Seq { len: Some(2) },
                Token::Bool(false),
                Token::Bool(false),
                Token::SeqEnd,
                Token::MapEnd,
                Token::TupleEnd,
                Token::StructEnd,
            ],
        );
    }
}
