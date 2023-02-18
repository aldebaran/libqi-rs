use crate::{num_bool::*, tuple::*, typing::Type, Dynamic, Map, Raw, String, Unit};
use derive_more::{From, TryInto};
use serde::{Deserialize, Serialize};

/// The [`Value`] structure represents the `value` type in the `qi` format and
/// is is an enumeration of every types of values defined in the format.
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
/// `Value`s cannot be deserialized from the `qi` format directly, because the `qi` format is not
/// self-describing, and `value` deserialization requires type information.
///
/// This is what the [`Dynamic`] type is for. If you want to deserialize a value from the
/// format, deserialize an [`Dynamic`] instead and then convert it into a `Value`.
// TODO: insert example here
///
/// `Value`s may however be deserialized from a self-describing format.
// TODO: insert example here
#[derive(Clone, From, TryInto, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[try_into(owned, ref, ref_mut)]
pub enum Value<'v> {
    Unit(Unit),
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
    Dynamic(Box<Dynamic<'v>>),
}

impl<'v> Value<'v> {
    pub fn unit() -> Self {
        Self::Tuple(Tuple::unit())
    }

    pub(crate) fn is_assignable_to_value_type(&self, t: &Type) -> bool {
        // Any value is assignable to dynamic.
        if t == &Type::Dynamic {
            return true;
        }

        match self {
            Value::Unit(_) => t == &Type::Unit,
            Value::Bool(_) => t == &Type::Bool,
            Value::Number(n) => n.is_assignable_to_value_type(t),
            Value::String(_) => t == &Type::String,
            Value::Raw(_) => t == &Type::Raw,
            Value::Option(option) => match t {
                Type::Option(t) => match option.as_ref() {
                    Some(value) => value.is_assignable_to_value_type(t),
                    None => true, // no value, could be assigned to anything.
                },
                _ => false,
            },
            Value::List(list) => match t {
                Type::List(t) => list
                    .iter()
                    .all(|element| element.is_assignable_to_value_type(t)),
                _ => false,
            },
            Value::Map(map) => match t {
                Type::Map {
                    key: key_type,
                    value: value_type,
                } => map.iter().all(|(key, value)| {
                    key.is_assignable_to_value_type(key_type)
                        && value.is_assignable_to_value_type(value_type)
                }),
                _ => false,
            },
            Value::Tuple(tuple) => match t {
                Type::Tuple(tuple_type) => {
                    if tuple.len() != tuple_type.len() {
                        return false;
                    }
                    tuple.elements().iter().zip(tuple_type.element_types()).all(
                        |(element, element_type)| element.is_assignable_to_value_type(element_type),
                    )
                }
                _ => false,
            },
            Value::Dynamic(dynamic) => dynamic.is_assignable_to_value_type(t),
        }
    }

    // TODO: as_XXX functions
    //
    pub fn as_unit(&self) -> std::option::Option<Unit> {
        match self {
            Self::Tuple(t) if t.is_unit() => Some(Unit),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> std::option::Option<Bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> std::option::Option<Number> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

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

    pub fn as_dynamic(&self) -> std::option::Option<&Dynamic> {
        match self {
            Self::Dynamic(d) => Some(d),
            _ => None,
        }
    }
}

impl<'v> Default for Value<'v> {
    fn default() -> Self {
        Self::unit()
    }
}

impl<T> From<T> for Value<'_>
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

/// Converts an option into a value.
///
/// # Example
/// ```
/// # use qi_format::{Value, String};
/// let opt = Some(Value::from(String::from("abc")));
/// assert_eq!(Value::from(opt.clone()),
///            Value::Option(Box::new(opt)));
/// ```
impl<'v> From<Option<'v>> for Value<'v> {
    fn from(o: Option<'v>) -> Self {
        Self::Option(Box::new(o))
    }
}

impl<'v> std::fmt::Display for Value<'v> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Unit(u) => u.fmt(f),
            Value::Bool(b) => b.fmt(f),
            Value::Number(n) => n.fmt(f),
            Value::String(s) => s.fmt(f),
            Value::Raw(r) => r.fmt(f),
            Value::Option(o) => match o.as_ref() {
                Some(v) => write!(f, "some({v})"),
                None => f.write_str("none"),
            },
            Value::List(l) => {
                let mut add_sep = false;
                for element in l {
                    if add_sep {
                        f.write_str(", ")?;
                    }
                    element.fmt(f)?;
                    add_sep = true;
                }
                Ok(())
            }
            Value::Map(m) => m.fmt(f),
            Value::Tuple(t) => t.fmt(f),
            Value::Dynamic(d) => d.fmt(f),
        }
    }
}

impl<'v> Serialize for Value<'v> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Unit(u) => u.serialize(serializer),
            Value::Bool(b) => b.serialize(serializer),
            Value::Number(n) => n.serialize(serializer),
            Value::String(s) => s.serialize(serializer),
            Value::Raw(r) => r.serialize(serializer),
            Value::Option(o) => o.serialize(serializer),
            Value::List(l) => l.serialize(serializer),
            Value::Map(m) => m.serialize(serializer),
            Value::Tuple(tuple) => tuple.serialize(serializer),
            Value::Dynamic(d) => d.serialize(serializer),
        }
    }
}

struct ValueVisitor;
impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = Value<'de>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Number::from(v)))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let str = v.encode_utf8(&mut [0; 4]).to_owned();
        Ok(Value::from(String::from(str)))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(String::from(v.to_owned())))
    }

    fn visit_string<E>(self, v: std::string::String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(String::from(v)))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::String(v.into()))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Raw::from(v.to_owned())))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Raw::from(v)))
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Raw::from(v)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(None))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Value::from(Some(value)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut list = match seq.size_hint() {
            Some(size) => List::with_capacity(size),
            None => List::new(),
        };
        while let Some(element) = seq.next_element()? {
            list.push(element);
        }
        Ok(Value::from(list))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut pair_vec = match map.size_hint() {
            Some(size) => Vec::with_capacity(size),
            None => Vec::new(),
        };
        while let Some((key, value)) = map.next_entry()? {
            pair_vec.push((key, value));
        }
        Ok(Value::from(Map::from_pair_elements(pair_vec)))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::unit())
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Value::from(Tuple::from_elements(vec![value])))
    }
}

impl<'de> Deserialize<'de> for Value<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

pub type Option<'v> = std::option::Option<Value<'v>>;

pub type List<'v> = Vec<Value<'v>>;

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

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
}
