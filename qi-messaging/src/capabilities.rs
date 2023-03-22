use derive_more::{From, Into};
use qi_types::{Dynamic, Map};

type MapImpl = Map<String, Dynamic>;

#[derive(
    Default, Clone, PartialEq, Eq, Debug, From, Into, serde::Serialize, serde::Deserialize,
)]
pub struct CapabilityMap(MapImpl);

impl CapabilityMap {
    pub fn new() -> Self {
        Self(Map::new())
    }

    pub fn set_capability(&mut self, name: &str, value: bool) -> &mut Self {
        self.0.insert(name.into(), Dynamic::from(value));
        self
    }

    pub fn iter(&self) -> Iter {
        Iter {
            iter: (&self.0).into_iter(),
        }
    }

    pub fn has_capability(&self, key: &str) -> bool {
        let item = self
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None });
        item == Some(true)
    }

    pub fn merged_with(&self, other: &Self) -> Self {
        let mut res = CapabilityMap::new();
        for (capability, enabled) in self.iter() {
            res.set_capability(capability, enabled && other.has_capability(capability));
        }
        for (capability, enabled) in other.iter() {
            res.set_capability(capability, enabled && self.has_capability(capability));
        }
        res
    }
}

impl<'a> std::iter::IntoIterator for &'a CapabilityMap {
    type Item = <Iter<'a> as std::iter::Iterator>::Item;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct Iter<'a> {
    iter: <&'a MapImpl as IntoIterator>::IntoIter,
}

impl<'a> std::iter::Iterator for Iter<'a> {
    type Item = (&'a String, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next();
        item.map(|(k, v)| (k, v.as_bool().expect("capability map value must be a bool")))
    }
}

impl<K, V> std::iter::FromIterator<(K, V)> for CapabilityMap
where
    K: Into<String>,
    V: Into<bool>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(Map::from_iter(
            iter.into_iter()
                .map(|(k, v)| (k.into(), Dynamic::from(v.into()))),
        ))
    }
}

pub const CLIENT_SERVER_SOCKET: &str = "ClientServerSocket";
pub const MESSAGE_FLAGS: &str = "MessageFlags";
pub const REMOTE_CANCELABLE_CALLS: &str = "RemoteCancelableCalls";
pub const OBJECT_PTR_UID: &str = "ObjectPtrUID";
pub const RELATIVE_ENDPOINT_URI: &str = "RelativeEndpointURI";

pub fn local_capabilities() -> CapabilityMap {
    CapabilityMap::from_iter([
        (CLIENT_SERVER_SOCKET, true),
        (MESSAGE_FLAGS, true),
        (REMOTE_CANCELABLE_CALLS, true),
        (OBJECT_PTR_UID, true),
        (RELATIVE_ENDPOINT_URI, true),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge() {
        let mut m1 = CapabilityMap::from_iter([
            ("A", true),
            ("B", true),
            ("C", false),
            ("D", false),
            ("E", true),
            ("F", false),
        ]);
        let m2 = CapabilityMap::from_iter([
            ("A", true),
            ("B", false),
            ("C", true),
            ("D", false),
            ("G", true),
            ("H", false),
        ]);
        let m = m1.merged_with(&m2);
        assert_eq!(m.has_capability("A"), true);
        assert_eq!(m.has_capability("B"), false);
        assert_eq!(m.has_capability("C"), false);
        assert_eq!(m.has_capability("D"), false);
        assert_eq!(m.has_capability("E"), false);
        assert_eq!(m.has_capability("F"), false);
        assert_eq!(m.has_capability("G"), false);
        assert_eq!(m.has_capability("H"), false);
    }
}
