use derive_more::{From, Index, Into};

/// The [`Map`] value represents an association of keys to values in the `qi` type system.
///
/// # Order
///
/// The key-value pairs have a consistent order that is determined by the sequence of insertion and
/// removal calls on the map. The order does not depend on the keys.
///
/// All iterators traverse the map in the order.
///
/// # Unicity of keys
///
/// This type guarantees the unicity of keys. When an insertion is done, if the key already exists
/// in the map, its value is overwritten with the inserted one.
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, From, Into, Index, Debug, Hash)]
pub struct Map<K, V>(Vec<(K, V)>);

impl<K, V> Map<K, V> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(size: usize) -> Self {
        Self(Vec::with_capacity(size))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut K, &mut V)> {
        self.0.iter_mut().map(|(k, v)| (k, v))
    }

    pub fn get<'s, Q>(&'s self, key: &Q) -> Option<&'s V>
    where
        Q: PartialEq<K> + ?Sized,
    {
        self.0.iter().find_map(|(k, v)| (key == k).then_some(v))
    }

    pub fn get_mut<'s, Q>(&'s mut self, key: &Q) -> Option<&'s mut V>
    where
        Q: PartialEq<K> + ?Sized,
    {
        self.0.iter_mut().find_map(|(k, v)| (key == k).then_some(v))
    }

    pub fn entry(&mut self, key: K) -> Entry<'_, K, V>
    where
        K: PartialEq,
    {
        let item = self
            .0
            .iter_mut()
            .enumerate()
            .find(|(_idx, (k, _v))| k == &key);
        match item {
            Some((idx, _pair)) => Entry::Occupied(OccupiedEntry {
                vec: &mut self.0,
                idx,
            }),
            None => Entry::Vacant(VacantEntry {
                key,
                vec: &mut self.0,
            }),
        }
    }

    fn position<Q>(&self, key: &Q) -> Option<usize>
    where
        Q: PartialEq<K> + ?Sized,
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

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.0.retain_mut(|(key, value)| f(key, value))
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: PartialEq<K> + ?Sized,
    {
        self.0.iter().any(|(key_in, _)| key == key_in)
    }
}

#[derive(Debug)]
pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

#[derive(Debug)]
pub struct OccupiedEntry<'a, K, V> {
    vec: &'a mut Vec<(K, V)>,
    idx: usize,
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.vec[self.idx].0
    }

    pub fn get(&self) -> &V {
        &self.vec[self.idx].1
    }

    pub fn get_mut(&mut self) -> &mut V {
        &mut self.vec[self.idx].1
    }

    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(self.get_mut(), value)
    }

    pub fn remove(self) -> V {
        self.remove_entry().1
    }

    pub fn remove_entry(self) -> (K, V) {
        self.vec.remove(self.idx)
    }
}

#[derive(Debug)]
pub struct VacantEntry<'a, K, V> {
    key: K,
    vec: &'a mut Vec<(K, V)>,
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn into_key(self) -> K {
        self.key
    }

    pub fn insert(self, value: V) -> &'a mut V {
        self.vec.push((self.key, value));
        &mut self.vec.last_mut().unwrap().1
    }
}

impl<K, V> std::iter::IntoIterator for Map<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, V> std::iter::IntoIterator for &'a Map<K, V> {
    type Item = &'a (K, V);
    type IntoIter = std::slice::Iter<'a, (K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, K, V> std::iter::IntoIterator for &'a mut Map<K, V> {
    type Item = &'a mut (K, V);
    type IntoIter = std::slice::IterMut<'a, (K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
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

#[cfg(test)]
mod tests {
    use super::*;
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
            &Map(vec![(32i16, "trente deux"), (34i16, "trente quatre")]),
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
