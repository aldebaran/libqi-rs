use crate::{ty, Dynamic, Type, Value};
use derive_more::{From, Index, Into, IntoIterator};

/// The [`Map`] value represents an association of keys to values in the `qi` type system.
///
/// This type guarantees the unicity of keys. When an insertion is done, if the key already exists
/// in the map, its value is overwritten with the inserted one.
#[derive(Default, Clone, PartialEq, Eq, From, Into, Index, IntoIterator, Debug)]
pub struct Map<K, V>(Vec<(K, V)>);

impl<K, V> Map<K, V> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.iter().map(|(k, _v)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.iter().map(|(_k, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    pub fn get<'s, Q>(&'s self, key: &Q) -> Option<&'s V>
    where
        Q: PartialEq<K>,
    {
        self.0
            .iter()
            .find_map(|(k, v)| if key == k { Some(v) } else { None })
    }

    fn position<Q>(&self, key: &Q) -> Option<usize>
    where
        Q: PartialEq<K>,
    {
        self.0.iter().position(|(k, _)| key == k)
    }

    pub fn insert(&mut self, key: K, mut value: V) -> Option<V>
    where
        K: PartialEq,
    {
        match self.position(&key) {
            Some(position) => {
                std::mem::swap(&mut value, &mut self.0[position].1);
                Some(value)
            }
            None => {
                self.0.push((key, value));
                None
            }
        }
    }

    fn get_type(&self) -> Type
    where
        K: ty::DynamicGetType,
        V: ty::DynamicGetType,
    {
        let common_types = self
            .iter()
            .map(|(key, value)| (Some(key.get_type()), Some(value.get_type())))
            .reduce(|(common_key, common_value), (key, value)| {
                (
                    ty::common_type(common_key, key),
                    ty::common_type(common_value, value),
                )
            });
        let (key, value) = match common_types {
            Some((key, value)) => (key, value),
            None => (None, None),
        };
        Type::Map {
            key: key.map(Box::new),
            value: value.map(Box::new),
        }
    }
}

impl<K, V> std::fmt::Display for Map<K, V>
where
    K: std::fmt::Display,
    V: std::fmt::Display,
{
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

impl<'a, K, V> std::iter::IntoIterator for &'a Map<K, V> {
    type Item = &'a (K, V);
    type IntoIter = std::slice::Iter<'a, (K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<K, V> std::iter::FromIterator<(K, V)> for Map<K, V>
where
    K: PartialEq,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut map = Map::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

impl<K, V> std::iter::Extend<(K, V)> for Map<K, V>
where
    K: PartialEq,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

impl<K, V> ty::StaticGetType for Map<K, V>
where
    K: ty::StaticGetType,
    V: ty::StaticGetType,
{
    fn get_type() -> crate::Type {
        ty::map_of(Some(K::get_type()), Some(V::get_type()))
    }
}

impl ty::DynamicGetType for Map<Dynamic, Dynamic> {
    fn get_type(&self) -> Type {
        self.get_type()
    }
}

impl ty::DynamicGetType for Map<Dynamic, Value> {
    fn get_type(&self) -> Type {
        self.get_type()
    }
}

impl ty::DynamicGetType for Map<Value, Dynamic> {
    fn get_type(&self) -> Type {
        self.get_type()
    }
}

impl ty::DynamicGetType for Map<Value, Value> {
    fn get_type(&self) -> Type {
        self.get_type()
    }
}

impl<K, V> serde::Serialize for Map<K, V>
where
    K: serde::Serialize,
    V: serde::Serialize,
{
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

impl<'de, K, V> serde::Deserialize<'de> for Map<K, V>
where
    K: serde::Deserialize<'de> + PartialEq,
    V: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<K, V>(std::marker::PhantomData<(K, V)>);
        impl<K, V> Visitor<K, V> {
            fn new() -> Self {
                Self(std::marker::PhantomData)
            }
        }
        impl<'de, K, V> serde::de::Visitor<'de> for Visitor<K, V>
        where
            K: serde::Deserialize<'de> + PartialEq,
            V: serde::Deserialize<'de>,
        {
            type Value = Map<K, V>;
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
                Ok(Map::from_iter(values))
            }
        }
        deserializer.deserialize_map(Visitor::new())
    }
}

#[macro_export]
macro_rules! map {
    ($($k:expr => $v:expr),+ $(,)*) => {
        $crate::Map::from_iter([$(($k, $v)),+])
    };
    () => {
        $crate::Map::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_map_from_iter_removes_duplicates() {
        assert_eq!(
            Map::from_iter([(42, "forty-two"), (13, "thirteen"), (42, "quarante-deux")]),
            Map::from_iter([(42, "quarante-deux"), (13, "thirteen")]),
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
}
