use crate::value::Dynamic;
use std::{borrow::Borrow, cmp::Ordering, collections::HashMap, hash::Hash};

type MapImpl = HashMap<String, Dynamic<bool>>;

#[derive(Default, Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct CapabilitiesMap(MapImpl);

impl CapabilitiesMap {
    pub fn new() -> Self {
        Self(MapImpl::new())
    }

    pub fn set_capability<K, V>(&mut self, name: K, value: V)
    where
        K: Into<String>,
        V: Into<bool>,
    {
        self.0.insert(name.into(), Dynamic(value.into()));
    }

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    pub fn get<K>(&self, key: &K) -> Option<&bool>
    where
        String: Borrow<K>,
        K: Hash + Eq + ?Sized,
    {
        self.0.get(key).map(|Dynamic(b)| b)
    }

    pub fn intersect(&mut self, other: &Self) -> &mut Self {
        for (key, other_value) in other.iter() {
            if let Some(value) = self.0.get_mut(key) {
                // Prefer values from this map when no ordering can be made. Only use the other map
                // values if they are strictly inferior.
                if let Some(Ordering::Less) = other_value.partial_cmp(value) {
                    *value = *other_value;
                }
            }
        }

        // Only keep capabilities that were present in `other`.
        self.0.retain(|k, _| other.get(k).is_some());

        self
    }
}

impl<'map> std::iter::IntoIterator for &'map CapabilitiesMap {
    type Item = <&'map MapImpl as IntoIterator>::Item;
    type IntoIter = <&'map MapImpl as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<K, V> std::iter::FromIterator<(K, V)> for CapabilitiesMap
where
    K: Into<String>,
    V: Into<bool>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(MapImpl::from_iter(
            iter.into_iter().map(|(k, v)| (k.into(), Dynamic(v.into()))),
        ))
    }
}

impl<K, V> std::iter::Extend<(K, V)> for CapabilitiesMap
where
    K: Into<String>,
    V: Into<bool>,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.0
            .extend(iter.into_iter().map(|(k, v)| (k.into(), Dynamic(v.into()))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn test_capability_map_merge_with() {
        let mut m = CapabilitiesMap::from_iter([
            ("A", true),
            ("B", true),
            ("C", false),
            ("D", false),
            ("E", true),
            ("F", false),
        ]);
        let m2 = CapabilitiesMap::from_iter([
            ("A", true),
            ("B", false),
            ("C", true),
            ("D", false),
            ("G", true),
            ("H", false),
        ]);
        m.intersect(&m2);
        assert_matches!(m.get("A"), Some(true));
        assert_matches!(m.get("B"), Some(false));
        assert_matches!(m.get("C"), Some(false));
        assert_matches!(m.get("D"), Some(false));
        assert_matches!(m.get("E"), None);
        assert_matches!(m.get("F"), None);
        assert_matches!(m.get("G"), None);
        assert_matches!(m.get("H"), None);
        assert_matches!(m.get("I"), None);
    }
}
