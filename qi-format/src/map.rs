use crate::Value;
use derive_more::{From, Index, Into, IntoIterator};
use std::iter::FromIterator;

/// The [`Map`] value represents an association of keys to values in the `qi` format. Both keys and
/// values are `Value`s.
///
/// # Unicity and order of keys
///
/// This type does *not* guarantee unicity of keys in the map. This means that if a map value is
/// read from the `qi` format contains multiple equivalent keys, these keys will be duplicated in
/// the resulting `Map` value.
#[derive(
    Default, Clone, PartialEq, Eq, From, Into, Index, IntoIterator, Hash, Debug,
)]
#[into_iterator(owned, ref, ref_mut)]
pub struct Map<'v>(Vec<(Value<'v>, Value<'v>)>);

impl<'v> Map<'v> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_pair_elements(v: Vec<(Value<'v>, Value<'v>)>) -> Self {
        Self(v)
    }

    pub fn keys(&self) -> impl Iterator<Item = &Value> {
        self.0.iter().map(|(k, _v)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.0.iter().map(|(_k, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.0.iter().map(|(k, v)| (k, v))
    }
}

impl<'v> std::fmt::Display for Map<'v> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;
        let mut add_sep = false;
        for (key, value) in &self.0 {
            if add_sep {
                f.write_str(", ")?;
            }
            write!(f, "{key}: {value}")?;
            add_sep = true;
        }
        f.write_str("}")
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

impl<'v> serde::Serialize for Map<'v> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut serializer = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            serializer.serialize_entry(key, value)?;
        }
        serializer.end()
    }
}

impl<'de> serde::Deserialize<'de> for Map<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Map<'de>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut values = match map.size_hint() {
                    Some(size) => Vec::with_capacity(size),
                    None => Vec::new(),
                };
                while let Some((key, value)) = map.next_entry()? {
                    values.push((key, value))
                }
                Ok(Map::from_pair_elements(values))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

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
}
