use crate::{
    list_ty, map_ty,
    num_bool::*,
    option_ty,
    tuple::*,
    ty::{self, Type},
    FormatterExt, List, Map, Object, Raw, Signature, Value,
};

/// [`Dynamic`] represents a `dynamic` value in the `qi` type system.
///
/// It is a value associated with its type information.
///
/// It is represented in the format as a value prepended with its type signature.
#[derive(Default, Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Dynamic(ValueWithType);

impl Dynamic {
    pub fn new(value: Value, t: Type) -> Result<Self, TypeMismatchError> {
        Ok(Self(ValueWithType::new(value, t)?))
    }
}

impl std::fmt::Display for Dynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ty::DynamicGetType for Dynamic {
    fn get_type(&self) -> Type {
        self.0.get_type()
    }
}

/// A value with additional type information.
#[derive(Clone, PartialEq, Eq, Debug)]
enum ValueWithType {
    Unit,
    Bool(bool),
    Number(Number),
    String(String),
    Raw(Raw),
    Option(OptionWithType),
    List(ListWithType),
    Map(MapWithType),
    Tuple(TupleWithType),
    Object(Box<Object>),
    Dynamic(Box<Dynamic>),
}

impl ValueWithType {
    pub fn new(value: Value, t: Type) -> Result<Self, TypeMismatchError> {
        use ty::DynamicGetType;
        if !value.is_assignable_to(&t) {
            return Err(TypeMismatchError {
                expected: t,
                actual: value.get_type(),
            });
        }
        let value = match value {
            Value::Unit => Self::Unit,
            Value::Bool(b) => Self::Bool(b),
            Value::Number(n) => Self::Number(n),
            Value::String(s) => Self::String(s),
            Value::Raw(r) => Self::Raw(r),
            Value::Object(o) => Self::Object(o),
            Value::Dynamic(d) => Self::Dynamic(d),
            Value::Option(option) => {
                let value_type = match t {
                    Type::Option(o) => o.as_deref().cloned(),
                    _ => unreachable!(),
                };
                Self::Option(OptionWithType(*option, value_type))
            }
            Value::List(list) => {
                let value_type = match t {
                    Type::List(l) => l.as_deref().cloned(),
                    _ => unreachable!(),
                };
                Self::List(ListWithType(list, value_type))
            }
            Value::Map(map) => {
                let (key_type, value_type) = match t {
                    Type::Map { key, value } => {
                        (key.as_deref().cloned(), value.as_deref().cloned())
                    }
                    _ => unreachable!(),
                };
                Self::Map(MapWithType {
                    value: map,
                    key_type,
                    value_type,
                })
            }
            Value::Tuple(tuple) => {
                let tuple_type = match t {
                    Type::Tuple(tuple_type) => tuple_type,
                    _ => unreachable!(),
                };
                Self::Tuple(TupleWithType(tuple, tuple_type))
            }
        };
        Ok(value)
    }

    pub fn into_value(self) -> Value {
        match self {
            Self::Unit => Value::Unit,
            Self::Bool(b) => Value::Bool(b),
            Self::Number(n) => Value::Number(n),
            Self::String(s) => Value::String(s),
            Self::Raw(r) => Value::Raw(r),
            Self::Option(o) => o.into_value(),
            Self::List(l) => l.into_value(),
            Self::Map(m) => m.into_value(),
            Self::Tuple(t) => t.into_value(),
            Self::Object(o) => Value::Object(o),
            Self::Dynamic(d) => Value::Dynamic(d),
        }
    }
}

impl Default for ValueWithType {
    fn default() -> Self {
        Self::Unit
    }
}

impl ty::DynamicGetType for ValueWithType {
    fn get_type(&self) -> Type {
        match self {
            Self::Unit => ().get_type(),
            Self::Bool(b) => b.get_type(),
            Self::Number(n) => n.get_type(),
            Self::String(s) => s.get_type(),
            Self::Raw(r) => r.get_type(),
            Self::Option(o) => o.get_type(),
            Self::List(l) => l.get_type(),
            Self::Map(m) => m.get_type(),
            Self::Tuple(t) => t.get_type(),
            Self::Object(o) => o.get_type(),
            Self::Dynamic(d) => d.get_type(),
        }
    }
}

impl std::fmt::Display for ValueWithType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Unit => f.write_str("()"),
            Self::Bool(b) => b.fmt(f),
            Self::Number(n) => n.fmt(f),
            Self::String(s) => s.fmt(f),
            Self::Raw(r) => f.write_raw(r),
            Self::Option(o) => o.fmt(f),
            Self::List(l) => l.fmt(f),
            Self::Map(m) => m.fmt(f),
            Self::Tuple(t) => t.fmt(f),
            Self::Object(o) => o.fmt(f),
            Self::Dynamic(d) => d.fmt(f),
        }
    }
}

fn serialize_signed_value<S, T, V>(serializer: S, t: T, value: &V) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
    T: Into<Option<Type>>,
    V: serde::Serialize,
{
    use serde::ser::SerializeTuple;
    let mut serializer = serializer.serialize_tuple(2)?;
    serializer.serialize_element(&Signature::new(t.into()))?;
    serializer.serialize_element(value)?;
    serializer.end()
}

impl serde::Serialize for ValueWithType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use ty::DynamicGetType;
        match self {
            Self::Unit => serialize_signed_value(serializer, ().get_type(), &()),
            Self::Bool(b) => serialize_signed_value(serializer, b.get_type(), b),
            Self::Number(n) => serialize_signed_value(serializer, n.get_type(), n),
            Self::String(s) => serialize_signed_value(serializer, s.get_type(), s),
            Self::Raw(r) => serialize_signed_value(serializer, r.get_type(), r),
            Self::Option(o) => o.serialize(serializer),
            Self::List(l) => l.serialize(serializer),
            Self::Map(m) => m.serialize(serializer),
            Self::Tuple(t) => t.serialize(serializer),
            Self::Object(o) => serialize_signed_value(serializer, o.get_type(), o.as_ref()),
            Self::Dynamic(d) => serialize_signed_value(serializer, d.get_type(), d.as_ref()),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ValueWithType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = ValueWithType;

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
                    .next_element_seed(ValueWithTypeSeed(value_type))?
                    .ok_or_else(|| invalid_length(1))?;

                Ok(value)
            }
        }

        deserializer.deserialize_tuple(2, Visitor)
    }
}

struct ValueWithTypeSeed(Option<Type>);

impl<'de> serde::de::DeserializeSeed<'de> for ValueWithTypeSeed {
    type Value = ValueWithType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        let value = match self.0 {
            Some(t) => match t {
                Type::Unit => {
                    <()>::deserialize(deserializer)?;
                    ValueWithType::Unit
                }
                Type::Bool => {
                    let v = bool::deserialize(deserializer)?;
                    ValueWithType::Bool(v)
                }
                Type::Int8 => {
                    let v = i8::deserialize(deserializer)?;
                    ValueWithType::Number(Number::Int8(v))
                }
                Type::UInt8 => {
                    let v = u8::deserialize(deserializer)?;
                    ValueWithType::Number(Number::UInt8(v))
                }
                Type::Int16 => {
                    let v = i16::deserialize(deserializer)?;
                    ValueWithType::Number(Number::Int16(v))
                }
                Type::UInt16 => {
                    let v = u16::deserialize(deserializer)?;
                    ValueWithType::Number(Number::UInt16(v))
                }
                Type::Int32 => {
                    let v = i32::deserialize(deserializer)?;
                    ValueWithType::Number(Number::Int32(v))
                }
                Type::UInt32 => {
                    let v = u32::deserialize(deserializer)?;
                    ValueWithType::Number(Number::UInt32(v))
                }
                Type::Int64 => {
                    let v = i64::deserialize(deserializer)?;
                    ValueWithType::Number(Number::Int64(v))
                }
                Type::UInt64 => {
                    let v = u64::deserialize(deserializer)?;
                    ValueWithType::Number(Number::UInt64(v))
                }
                Type::Float32 => {
                    let v = Float32::deserialize(deserializer)?;
                    ValueWithType::Number(Number::Float32(v))
                }
                Type::Float64 => {
                    let v = Float64::deserialize(deserializer)?;
                    ValueWithType::Number(Number::Float64(v))
                }
                Type::String => {
                    let v = String::deserialize(deserializer)?;
                    ValueWithType::String(v)
                }
                Type::Raw => {
                    let v = Raw::deserialize(deserializer)?;
                    ValueWithType::Raw(v)
                }
                Type::Object => {
                    let v = Object::deserialize(deserializer)?;
                    ValueWithType::Object(Box::new(v))
                }
                Type::Option(t) => {
                    let v = OptionWithTypeSeed(t.as_deref().cloned()).deserialize(deserializer)?;
                    ValueWithType::Option(v)
                }
                Type::List(t) | Type::VarArgs(t) => {
                    let v = ListWithTypeSeed(t.as_deref().cloned()).deserialize(deserializer)?;
                    ValueWithType::List(v)
                }
                Type::Map { key, value } => {
                    let v = MapWithTypeSeed {
                        key: key.as_deref().cloned(),
                        value: value.as_deref().cloned(),
                    }
                    .deserialize(deserializer)?;
                    ValueWithType::Map(v)
                }
                Type::Tuple(tuple) => {
                    let v = TupleWithTypeSeed(tuple).deserialize(deserializer)?;
                    ValueWithType::Tuple(v)
                }
            },
            None => {
                let v = Dynamic::deserialize(deserializer)?;
                ValueWithType::Dynamic(Box::new(v))
            }
        };
        Ok(value)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct OptionWithType(Option<Value>, Option<Type>);

impl OptionWithType {
    fn into_value(self) -> Value {
        Value::Option(Box::new(self.0))
    }
}

impl ty::DynamicGetType for OptionWithType {
    fn get_type(&self) -> Type {
        Type::Option(self.1.clone().map(Box::new))
    }
}

impl std::fmt::Display for OptionWithType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_option(&self.0)
    }
}

impl serde::Serialize for OptionWithType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_signed_value(serializer, option_ty!(self.1.clone()), &self.0)
    }
}

struct OptionWithTypeSeed(Option<Type>);

impl<'de> serde::de::DeserializeSeed<'de> for OptionWithTypeSeed {
    type Value = OptionWithType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor(Option<Type>);
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = OptionWithType;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an optional value")
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::de::DeserializeSeed;
                let typed_value = ValueWithTypeSeed(self.0.clone()).deserialize(deserializer)?;
                // Drop the type information and transform into a simple value. The type
                // information is already stored with the optional.
                let value = typed_value.into_value();
                Ok(OptionWithType(Some(value), self.0))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OptionWithType(None, self.0))
            }
        }
        deserializer.deserialize_option(Visitor(self.0))
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ListWithType(List<Value>, Option<Type>);

impl ListWithType {
    fn into_value(self) -> Value {
        Value::List(self.0)
    }
}

impl ty::DynamicGetType for ListWithType {
    fn get_type(&self) -> Type {
        Type::List(self.1.clone().map(Box::new))
    }
}

impl std::fmt::Display for ListWithType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_list(&self.0)
    }
}

impl serde::Serialize for ListWithType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_signed_value(serializer, list_ty!(self.1.clone()), &self.0)
    }
}

struct ListWithTypeSeed(Option<Type>);

impl<'de> serde::de::DeserializeSeed<'de> for ListWithTypeSeed {
    type Value = ListWithType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor(Option<Type>);
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = ListWithType;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a list or varargs value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut list = List::new();
                while let Some(typed_value) =
                    seq.next_element_seed(ValueWithTypeSeed(self.0.clone()))?
                {
                    // Drop the type information and transform into a simple value. The type
                    // information is already stored with the list.
                    list.push(typed_value.into_value());
                }
                Ok(ListWithType(list, self.0))
            }
        }
        deserializer.deserialize_seq(Visitor(self.0))
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct MapWithType {
    value: Map<Value, Value>,
    key_type: Option<Type>,
    value_type: Option<Type>,
}

impl MapWithType {
    fn into_value(self) -> Value {
        Value::Map(self.value)
    }
}

impl ty::DynamicGetType for MapWithType {
    fn get_type(&self) -> Type {
        Type::Map {
            key: self.key_type.clone().map(Box::new),
            value: self.value_type.clone().map(Box::new),
        }
    }
}

impl std::fmt::Display for MapWithType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl serde::Serialize for MapWithType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_signed_value(
            serializer,
            map_ty!(self.key_type.clone(), self.value_type.clone()),
            &self.value,
        )
    }
}

struct MapWithTypeSeed {
    key: Option<Type>,
    value: Option<Type>,
}

impl<'de> serde::de::DeserializeSeed<'de> for MapWithTypeSeed {
    type Value = MapWithType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor {
            key: Option<Type>,
            value: Option<Type>,
        }
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = MapWithType;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut pair_vec = Vec::new();
                while let Some((key, value)) = map.next_entry_seed(
                    ValueWithTypeSeed(self.key.clone()),
                    ValueWithTypeSeed(self.value.clone()),
                )? {
                    // Drop the type information and transform into simple values. The types
                    // information are already stored with the map.
                    pair_vec.push((key.into_value(), value.into_value()));
                }
                Ok(MapWithType {
                    value: Map::from_iter(pair_vec),
                    key_type: self.key,
                    value_type: self.value,
                })
            }
        }
        deserializer.deserialize_map(Visitor {
            key: self.key,
            value: self.value,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct TupleWithType(Tuple, ty::TupleType);

impl TupleWithType {
    fn into_value(self) -> Value {
        Value::Tuple(self.0)
    }
}

impl ty::DynamicGetType for TupleWithType {
    fn get_type(&self) -> Type {
        Type::Tuple(self.1.clone())
    }
}

impl std::fmt::Display for TupleWithType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl serde::Serialize for TupleWithType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_signed_value(serializer, Type::Tuple(self.1.clone()), &self.0)
    }
}

struct TupleWithTypeSeed(ty::TupleType);

impl<'de> serde::de::DeserializeSeed<'de> for TupleWithTypeSeed {
    type Value = TupleWithType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn deser_tuple_from_seq<'de, A, I, E>(
            mut seq: A,
            element_types: I,
            expecting: &E,
        ) -> Result<Tuple, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
            I: IntoIterator<Item = Option<Type>>,
            E: serde::de::Expected,
        {
            let mut elements = Vec::new();
            for element_type in element_types {
                match seq.next_element_seed(ValueWithTypeSeed(element_type))? {
                    Some(element) => elements.push(element.into_value()),
                    None => {
                        return Err(serde::de::Error::invalid_length(elements.len(), expecting))
                    }
                };
            }
            Ok(Tuple::from_vec(elements))
        }

        struct TupleVisitor(ty::TupleType);
        impl<'de> serde::de::Visitor<'de> for TupleVisitor {
            type Value = TupleWithType;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a tuple value of size {len}", len = self.0.len())
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let tuple = deser_tuple_from_seq(seq, self.0.element_types(), &self)?;
                Ok(TupleWithType(tuple, self.0))
            }
        }

        struct StructVisitor(String, Vec<ty::StructField>);
        impl<'de> serde::de::Visitor<'de> for StructVisitor {
            type Value = TupleWithType;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a struct value of size {len}", len = self.1.len(),)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let tuple = deser_tuple_from_seq(
                    seq,
                    self.1.iter().map(|field| field.value_type.clone()),
                    &self,
                )?;
                Ok(TupleWithType(tuple, ty::TupleType::Struct(self.0, self.1)))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut elements = Vec::new();
                for field in &self.1 {
                    match map.next_entry_seed(
                        std::marker::PhantomData::<&str>,
                        ValueWithTypeSeed(field.value_type.clone()),
                    )? {
                        Some((key, value)) if key == field.name => {
                            elements.push(value.into_value())
                        }
                        Some(_) => {
                            return Err(serde::de::Error::custom("missing field `{field.name}`"))
                        }
                        None => {
                            return Err(serde::de::Error::invalid_length(elements.len(), &self))
                        }
                    };
                }

                let tuple = Tuple::from_vec(elements);
                Ok(TupleWithType(tuple, ty::TupleType::Struct(self.0, self.1)))
            }
        }

        match self.0 {
            ty::TupleType::Tuple(_) | ty::TupleType::TupleStruct(_, _) => {
                deserializer.deserialize_tuple(self.0.len(), TupleVisitor(self.0))
            }
            ty::TupleType::Struct(name, fields) => {
                deserializer.deserialize_tuple(fields.len(), StructVisitor(name, fields))
            }
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
#[error("type mismatch error, expected {expected}, got {actual}")]
pub struct TypeMismatchError {
    expected: Type,
    actual: Type,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{list_ty, map_ty, option_ty, struct_ty};
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_dynamic_serde() {
        let value_type = struct_ty! {
            MyStruct {
                an_int: Type::Int32,
                a_raw: Type::Raw,
                an_option: option_ty!(map_ty!(Type::String, list_ty!(Type::Bool))),
            }
        };
        let value = Value::Tuple(Tuple::from_vec(vec![
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
        let dynamic = Dynamic::new(value, value_type).unwrap();
        assert_tokens(
            &dynamic,
            &[
                Token::Tuple { len: 2 },
                Token::Str("(ir+{s[b]})<MyStruct,an_int,a_raw,an_option>"),
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
                Token::TupleEnd,
            ],
        );
    }
}