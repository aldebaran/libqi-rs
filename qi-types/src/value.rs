use crate::{
    num_bool::*,
    tuple::*,
    ty::{self, Type},
    Dynamic, FormatterExt, List, Map, Object, Raw,
};
use derive_more::{From, TryInto};

/// The [`Value`] structure represents any value of `qi` type system and
/// is is an enumeration of every types of values.
#[derive(Clone, From, TryInto, PartialEq, Eq, Debug)]
pub enum Value {
    Unit,
    Bool(bool),
    #[from(ignore)]
    Number(Number),
    String(String),
    Raw(Raw),
    #[try_into(ignore)] // Conflicts with standard conversion T -> Opt<T>
    Option(Box<Option<Value>>),
    List(List<Value>),
    Map(Map<Value, Value>),
    Tuple(Tuple),
    Object(Box<Object>),
    Dynamic(Box<Dynamic>),
}

impl Value {
    pub fn as_unit(&self) -> Option<()> {
        match self {
            Self::Unit => Some(()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<Number> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_raw(&self) -> Option<&Raw> {
        match self {
            Self::Raw(r) => Some(r),
            _ => None,
        }
    }

    pub fn as_option(&self) -> Option<&Option<Value>> {
        match self {
            Self::Option(o) => Some(o.as_ref()),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&List<Value>> {
        match self {
            Self::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&Map<Value, Value>> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> Option<&Tuple> {
        match self {
            Self::Tuple(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Self::Object(o) => Some(o.as_ref()),
            _ => None,
        }
    }

    pub fn as_dynamic(&self) -> Option<&Dynamic> {
        match self {
            Self::Dynamic(d) => Some(d),
            _ => None,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Unit
    }
}

impl<T> From<T> for Value
where
    T: Into<Number>,
{
    fn from(v: T) -> Self {
        Value::Number(v.into())
    }
}

impl<'s> From<&'s str> for Value {
    fn from(s: &'s str) -> Self {
        Value::String(String::from(s))
    }
}

/// Converts an option into a value.
///
/// # Example
/// ```
/// # use qi_types::Value;
/// let opt = Some(Value::from(String::from("abc")));
/// assert_eq!(Value::from(opt.clone()),
///            Value::Option(Box::new(opt)));
/// ```
impl From<Option<Value>> for Value {
    fn from(o: Option<Value>) -> Self {
        Self::Option(Box::new(o))
    }
}

impl From<Object> for Value {
    fn from(o: Object) -> Self {
        Self::Object(Box::new(o))
    }
}

impl From<Dynamic> for Value {
    fn from(d: Dynamic) -> Self {
        Self::Dynamic(Box::new(d))
    }
}

impl ty::DynamicGetType for Value {
    fn ty(&self) -> Option<Type> {
        match self {
            Self::Unit => ().ty(),
            Self::Bool(b) => b.ty(),
            Self::Number(n) => Some(n.ty()),
            Self::String(s) => s.ty(),
            Self::Raw(r) => r.ty(),
            Self::Option(o) => o.ty(),
            Self::List(l) => l.ty(),
            Self::Map(m) => m.ty(),
            Self::Tuple(t) => t.ty(),
            Self::Object(o) => o.ty(),
            Self::Dynamic(d) => d.ty(),
        }
    }

    fn current_ty(&self) -> Type {
        match self {
            Self::Unit => ().current_ty(),
            Self::Bool(b) => b.current_ty(),
            Self::Number(n) => n.current_ty(),
            Self::String(s) => s.current_ty(),
            Self::Raw(r) => r.current_ty(),
            Self::Option(o) => o.current_ty(),
            Self::List(l) => l.current_ty(),
            Self::Map(m) => m.current_ty(),
            Self::Tuple(t) => t.current_ty(),
            Self::Object(o) => o.current_ty(),
            Self::Dynamic(d) => d.current_ty(),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Unit => f.write_str("()"),
            Self::Bool(b) => b.fmt(f),
            Self::Number(n) => n.fmt(f),
            Self::String(s) => s.fmt(f),
            Self::Raw(r) => f.write_raw(r),
            Self::Option(o) => f.write_option(o),
            Self::List(l) => f.write_list(l),
            Self::Map(m) => m.fmt(f),
            Self::Tuple(t) => t.fmt(f),
            Self::Object(o) => o.fmt(f),
            Self::Dynamic(d) => d.fmt(f),
        }
    }
}

impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Unit => ().serialize(serializer),
            Value::Bool(b) => b.serialize(serializer),
            Value::Number(n) => n.serialize(serializer),
            Value::String(s) => s.serialize(serializer),
            Value::Raw(r) => r.serialize(serializer),
            Value::Option(o) => o.serialize(serializer),
            Value::List(l) => l.serialize(serializer),
            Value::Map(m) => m.serialize(serializer),
            Value::Tuple(tuple) => tuple.serialize(serializer),
            Value::Object(object) => object.serialize(serializer),
            Value::Dynamic(d) => d.serialize(serializer),
        }
    }
}

struct ValueVisitor;

impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = Value;

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
        Ok(Value::from(str))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(v.to_owned()))
    }

    fn visit_string<E>(self, v: std::string::String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::from(Raw::copy_from_slice(v)))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
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
        use serde::Deserialize;
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
        Ok(Value::from(Map::from_iter(pair_vec)))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Unit)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        let value = Value::deserialize(deserializer)?;
        Ok(Value::from(Tuple::from_vec(vec![value])))
    }
}

impl<'de, 'v> serde::Deserialize<'de> for Value
where
    'de: 'v,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

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
