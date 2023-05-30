use crate::types::{self, Dynamic};
use derive_more::{From, Into};
use std::cmp::Ordering;

type MapImpl = types::Map<String, Dynamic>;

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    From,
    Into,
    serde::Serialize,
    serde::Deserialize,
)]
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

    pub fn get_capability<K>(&self, key: K) -> Option<&Dynamic>
    where
        K: PartialEq<String>,
    {
        self.0.get(&key)
    }

    pub fn has_flag_capability<K>(&self, key: K) -> bool
    where
        K: PartialEq<String>,
    {
        matches!(self.get_capability(key), Some(Dynamic::Bool(true)))
    }

    pub(crate) fn resolve_minimums_against<F>(&mut self, other: &Self, mut reset_default: F)
    where
        F: FnMut(&mut Dynamic),
    {
        use types::map::Entry;
        for (key, other_value) in other.iter() {
            match self.0.entry(key.clone()) {
                Entry::Occupied(mut entry) => {
                    // Prefer values from this map when no ordering can be made. Only use the other map
                    // values if they are strictly inferior.
                    if let Some(Ordering::Less) = other_value.partial_cmp(entry.get()) {
                        entry.insert(other_value.clone());
                    }
                }
                Entry::Vacant(entry) => {
                    // The value does not exist in this one but exists in the other, set them to
                    // the default.
                    let mut value = other_value.clone();
                    reset_default(&mut value);
                    entry.insert(value);
                }
            }
        }

        // Check for capabilities that were present in this one but not in the other, and reset
        // them to the default.
        for value in
            self.0
                .iter_mut()
                .filter_map(|(key, value)| match other.get_capability(key.as_str()) {
                    Some(_) => None,
                    None => Some(value),
                })
        {
            reset_default(value);
        }
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
        (&self.0).into_iter()
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

#[derive(Debug, thiserror::Error)]
#[error("expected key {0} to have value {1}")]
pub(crate) struct ExpectedKeyValueError<T>(String, T);

pub fn reset_to_default(value: &mut Dynamic) {
    match value {
        Dynamic::Unit => {}
        Dynamic::Bool(v) => *v = Default::default(),
        Dynamic::Number(v) => *v = Default::default(),
        Dynamic::String(v) => *v = Default::default(),
        Dynamic::Raw(v) => *v = Default::default(),
        Dynamic::Option(v) => *v = Default::default(),
        Dynamic::List(v) => *v = Default::default(),
        Dynamic::Map(v) => *v = Default::default(),
        Dynamic::Tuple(v) => *v = Default::default(),
        Dynamic::Object(v) => *v = Default::default(),
        Dynamic::Dynamic(v) => *v = Default::default(),
    }
}

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

pub(crate) fn local() -> Map {
    LOCAL_BASE_CAPABILITIES.to_map()
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
        m.resolve_minimums_against(&m2, reset_to_default);
        assert_matches!(m.get_capability("A"), Some(Dynamic::Bool(true)));
        assert_matches!(m.get_capability("B"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get_capability("C"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get_capability("D"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get_capability("E"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get_capability("F"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get_capability("G"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get_capability("H"), Some(Dynamic::Bool(false)));
        assert_matches!(m.get_capability("I"), None);
    }
}
