use crate::{number::*, Map, Object};
use bytes::Bytes;

/// The [`Value`] structure represents any value of `qi` type system and
/// is is an enumeration of every types of values.
#[derive(Clone, PartialEq, Debug, derive_more::From, serde::Serialize, serde::Deserialize)]
pub enum AnyValue {
    #[from]
    Unit,
    #[from]
    Bool(bool),
    #[from(forward)]
    Number(Number),
    #[from]
    String(String),
    #[from]
    Raw(Bytes),
    Option(Box<Option<AnyValue>>),
    #[from]
    List(Vec<AnyValue>),
    #[from]
    Map(Map<AnyValue, AnyValue>),
    #[serde(with = "tuple")]
    Tuple(Vec<AnyValue>),
    Object(Box<Object>),
}

impl Default for AnyValue {
    fn default() -> Self {
        Self::Unit
    }
}

impl From<&str> for AnyValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

/// Converts an option into a value.
///
/// # Example
/// ```
/// # use qi_value::AnyValue;
/// let opt = Some(AnyValue::from(String::from("abc")));
/// assert_eq!(AnyValue::from(opt.clone()),
///            AnyValue::Option(Box::new(opt)));
/// ```
impl From<Option<AnyValue>> for AnyValue {
    fn from(o: Option<AnyValue>) -> Self {
        Self::Option(Box::new(o))
    }
}

impl From<Object> for AnyValue {
    fn from(v: Object) -> Self {
        Self::Object(Box::new(v))
    }
}

mod tuple {
    use super::AnyValue;

    pub(super) fn serialize<S>(elements: &Vec<AnyValue>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut serializer = serializer.serialize_tuple(elements.len())?;
        for element in elements {
            serializer.serialize_element(element)?;
        }
        serializer.end()
    }

    // Tuples size is not known at compile time, which means we cannot provide it as information to
    // serde when deserializing a new tuple. We must rely on what the deserializer knows of the
    // value and information it can provide us.
    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<AnyValue>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Vec<AnyValue>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple value")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(vec![])
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = serde::Deserialize::deserialize(deserializer)?;
                Ok(vec![value])
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
                Ok(elements)
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
                    let element = AnyValue::Tuple(vec![key, value]);
                    elements.push(element);
                }
                Ok(elements)
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
