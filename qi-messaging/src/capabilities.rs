use crate::value::{Dynamic, Map};
use std::cmp::Ordering;

type MapImpl = Map<String, Dynamic>;

#[derive(
    Default, Clone, PartialEq, Eq, PartialOrd, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct CapabilitiesMap(MapImpl);

impl CapabilitiesMap {
    pub fn new() -> Self {
        Self(MapImpl::new())
    }

    pub fn set_capability<K, V>(&mut self, name: K, value: V)
    where
        K: Into<String>,
        V: Into<Dynamic>,
    {
        self.0.insert(name.into(), value.into());
    }

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    pub fn get<K>(&self, key: &K) -> Option<&Dynamic>
    where
        K: PartialEq<String> + ?Sized,
    {
        self.0.get(key)
    }

    pub fn has_flag_capability<K>(&self, key: &K) -> bool
    where
        K: PartialEq<String> + ?Sized,
    {
        matches!(self.get(key), Some(Dynamic::Bool(true)))
    }

    pub fn intersect(&mut self, other: &Self) -> &mut Self {
        for (key, other_value) in other.iter() {
            if let Some(value) = self.0.get_mut(key) {
                // Prefer values from this map when no ordering can be made. Only use the other map
                // values if they are strictly inferior.
                if let Some(Ordering::Less) = other_value.partial_cmp(value) {
                    *value = other_value.clone();
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
        (&self.0).into_iter()
    }
}

impl<K, V> std::iter::FromIterator<(K, V)> for CapabilitiesMap
where
    K: Into<String>,
    V: Into<Dynamic>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(MapImpl::from_iter(
            iter.into_iter().map(|(k, v)| (k.into(), v.into())),
        ))
    }
}

impl<K, V> std::iter::Extend<(K, V)> for CapabilitiesMap
where
    K: Into<String>,
    V: Into<Dynamic>,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.0
            .extend(iter.into_iter().map(|(k, v)| (k.into(), v.into())))
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
        assert_matches!(m.get("A"), Some(Dynamic::Bool(true)));
        assert_matches!(m.get("B"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get("C"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get("D"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get("E"), None);
        assert_matches!(m.get("F"), None);
        assert_matches!(m.get("G"), None);
        assert_matches!(m.get("H"), None);
        assert_matches!(m.get("I"), None);
    }
}
