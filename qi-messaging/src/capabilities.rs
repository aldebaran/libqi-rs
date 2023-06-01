use derive_more::{From, Into};
use once_cell::sync::OnceCell;
pub use qi_types::Dynamic;
use std::{borrow::Borrow, cmp::Ordering, collections::HashMap};

type MapImpl = HashMap<String, Dynamic>;

#[derive(Default, Clone, Debug, From, Into, serde::Serialize, serde::Deserialize)]
pub struct Map(MapImpl);

impl Map {
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
        String: Borrow<K>,
        K: std::hash::Hash + Eq + ?Sized,
    {
        self.0.get(key)
    }

    pub fn has_flag_capability<K>(&self, key: &K) -> bool
    where
        String: Borrow<K>,
        K: std::hash::Hash + Eq + ?Sized,
    {
        matches!(self.get(key), Some(Dynamic::Bool(true)))
    }

    pub(crate) fn intersect(&mut self, other: &Self) -> &mut Self {
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

    /// Checks that the capabilities have the required values that are only supported by this implementation.
    ///
    /// This implementation does not yet handle all the possible effects of each capability cases. This function
    /// ensures that the capabilities have the only values that are handle at the moment.
    pub(crate) fn check_required(&self) -> Result<&Self, ExpectedKeyValueError<bool>> {
        let base = Base::from_map(self);

        // TODO: Implement capabilities so that this function always succeeds, so that we may remove it.
        if !base.client_server_socket {
            return Err(ExpectedKeyValueError(
                Base::CLIENT_SERVER_SOCKET.into(),
                true,
            ));
        }
        if base.meta_object_cache {
            return Err(ExpectedKeyValueError(Base::META_OBJECT_CACHE.into(), false));
        }
        if !base.message_flags {
            return Err(ExpectedKeyValueError(Base::MESSAGE_FLAGS.into(), true));
        }
        if !base.remote_cancelable_calls {
            return Err(ExpectedKeyValueError(
                Base::REMOTE_CANCELABLE_CALLS.into(),
                true,
            ));
        }
        if !base.object_ptr_uid {
            return Err(ExpectedKeyValueError(Base::OBJECT_PTR_UID.into(), true));
        }
        if !base.relative_endpoint_uri {
            return Err(ExpectedKeyValueError(
                Base::RELATIVE_ENDPOINT_URI.into(),
                true,
            ));
        }
        Ok(self)
    }
}

impl<'a> std::iter::IntoIterator for &'a Map {
    type Item = <&'a MapImpl as IntoIterator>::Item;
    type IntoIter = <&'a MapImpl as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<K, V> std::iter::FromIterator<(K, V)> for Map
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

impl<K, V> std::iter::Extend<(K, V)> for Map
where
    K: Into<String>,
    V: Into<Dynamic>,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.0
            .extend(iter.into_iter().map(|(k, v)| (k.into(), v.into())))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, thiserror::Error)]
#[error("expected key {0} to have value {1}")]
pub(crate) struct ExpectedKeyValueError<T>(String, T);

#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct Base {
    pub client_server_socket: bool,
    pub meta_object_cache: bool,
    pub message_flags: bool,
    pub remote_cancelable_calls: bool,
    pub object_ptr_uid: bool,
    pub relative_endpoint_uri: bool,
}

impl Base {
    const CLIENT_SERVER_SOCKET: &'static str = "ClientServerSocket";
    const META_OBJECT_CACHE: &'static str = "MetaObjectCache";
    const MESSAGE_FLAGS: &'static str = "MessageFlags";
    const REMOTE_CANCELABLE_CALLS: &'static str = "RemoteCancelableCalls";
    const OBJECT_PTR_UID: &'static str = "ObjectPtrUID";
    const RELATIVE_ENDPOINT_URI: &'static str = "RelativeEndpointURI";

    pub fn from_map(map: &Map) -> Self {
        Self {
            client_server_socket: map.has_flag_capability(Self::CLIENT_SERVER_SOCKET),
            meta_object_cache: map.has_flag_capability(Self::META_OBJECT_CACHE),
            message_flags: map.has_flag_capability(Self::MESSAGE_FLAGS),
            remote_cancelable_calls: map.has_flag_capability(Self::REMOTE_CANCELABLE_CALLS),
            object_ptr_uid: map.has_flag_capability(Self::OBJECT_PTR_UID),
            relative_endpoint_uri: map.has_flag_capability(Self::RELATIVE_ENDPOINT_URI),
        }
    }

    pub fn to_map(self) -> Map {
        Map::from_iter([
            (Self::CLIENT_SERVER_SOCKET, self.client_server_socket),
            (Self::META_OBJECT_CACHE, self.meta_object_cache),
            (Self::MESSAGE_FLAGS, self.message_flags),
            (Self::REMOTE_CANCELABLE_CALLS, self.remote_cancelable_calls),
            (Self::OBJECT_PTR_UID, self.object_ptr_uid),
            (Self::RELATIVE_ENDPOINT_URI, self.relative_endpoint_uri),
        ])
    }
}
impl From<&Map> for Base {
    fn from(map: &Map) -> Self {
        Self::from_map(map)
    }
}

impl From<&Base> for Map {
    fn from(common: &Base) -> Self {
        common.to_map()
    }
}

const LOCAL_BASE_CAPABILITIES: Base = Base {
    client_server_socket: true,
    meta_object_cache: false, // Unsupported feature
    message_flags: true,
    remote_cancelable_calls: true,
    object_ptr_uid: true,
    relative_endpoint_uri: true,
};

static LOCAL_CAPABILITIES: OnceCell<Map> = OnceCell::new();

pub(crate) fn local() -> &'static Map {
    LOCAL_CAPABILITIES.get_or_init(|| LOCAL_BASE_CAPABILITIES.to_map())
}

pub(crate) fn local_intersected_with(remote: &Map) -> Result<Map, ExpectedKeyValueError<bool>> {
    let mut capabilities = local().clone();
    capabilities.intersect(remote).check_required()?;
    Ok(capabilities)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn test_capability_map_merge_with() {
        let mut m = Map::from_iter([
            ("A", true),
            ("B", true),
            ("C", false),
            ("D", false),
            ("E", true),
            ("F", false),
        ]);
        let m2 = Map::from_iter([
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
        assert_matches!(m.get("E"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get("F"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get("G"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get("H"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get("I"), None);
    }
}
