use once_cell::sync::Lazy;
use qi_value::{IntoValue, KeyDynValueMap};
use std::cmp::Ordering;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct Capabilities {
    remote_cancelable_calls: bool,
    object_ptr_uid: bool,
    relative_endpoint_uri: bool,
}

impl Capabilities {
    const REMOTE_CANCELABLE_CALLS: &'static str = "RemoteCancelableCalls";
    const OBJECT_PTR_UID: &'static str = "ObjectPtrUID";
    const RELATIVE_ENDPOINT_URI: &'static str = "RelativeEndpointURI";

    const fn new() -> Self {
        Self {
            remote_cancelable_calls: true,
            object_ptr_uid: true,
            relative_endpoint_uri: true,
        }
    }

    fn from_map(map: &KeyDynValueMap) -> Self {
        Self {
            remote_cancelable_calls: map.get_as(Self::REMOTE_CANCELABLE_CALLS).unwrap_or(false),
            object_ptr_uid: map.get_as(Self::OBJECT_PTR_UID).unwrap_or(false),
            relative_endpoint_uri: map.get_as(Self::RELATIVE_ENDPOINT_URI).unwrap_or(false),
        }
    }

    fn to_map(self) -> KeyDynValueMap {
        KeyDynValueMap::from_iter([
            (
                Self::REMOTE_CANCELABLE_CALLS.to_owned(),
                self.remote_cancelable_calls.into_value(),
            ),
            (
                Self::OBJECT_PTR_UID.to_owned(),
                self.object_ptr_uid.into_value(),
            ),
            (
                Self::RELATIVE_ENDPOINT_URI.to_owned(),
                self.relative_endpoint_uri.into_value(),
            ),
        ])
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, thiserror::Error)]
#[error("expected key \"{0}\" to have value \"{1}\"")]
pub(crate) struct KeyValueExpectError(String, bool);

/// Checks that the capabilities have the required values that are only supported by this implementation.
///
/// This implementation does not yet handle all the possible effects of each capability cases. This function
/// ensures that the capabilities have the only values that are handle at the moment.
pub(crate) fn check_required(map: &KeyDynValueMap) -> Result<Capabilities, KeyValueExpectError> {
    let capabilities = Capabilities::from_map(map);

    // TODO: Implement capabilities so that this function always succeeds, so that we may remove it.
    if !capabilities.remote_cancelable_calls {
        return Err(KeyValueExpectError(
            Capabilities::REMOTE_CANCELABLE_CALLS.into(),
            true,
        ));
    }
    if !capabilities.object_ptr_uid {
        return Err(KeyValueExpectError(
            Capabilities::OBJECT_PTR_UID.into(),
            true,
        ));
    }
    if !capabilities.relative_endpoint_uri {
        return Err(KeyValueExpectError(
            Capabilities::RELATIVE_ENDPOINT_URI.into(),
            true,
        ));
    }
    Ok(capabilities)
}

pub(crate) fn local_map() -> &'static KeyDynValueMap {
    const LOCAL_CAPABILITIES: Capabilities = Capabilities::new();
    static LOCAL_CAPABILITIES_MAP: Lazy<KeyDynValueMap> = Lazy::new(|| LOCAL_CAPABILITIES.to_map());
    &LOCAL_CAPABILITIES_MAP
}

fn intersect(this: &mut KeyDynValueMap, other: &KeyDynValueMap) {
    let this = this.as_hash_map_mut();
    let other = other.as_hash_map();
    for (key, other_value) in other {
        if let Some(value) = this.get_mut(key) {
            // Prefer values from this map when no ordering can be made. Only use the other map
            // values if they are strictly inferior.
            if let Some(Ordering::Less) = other_value.partial_cmp(value) {
                *value = other_value.clone().into_owned();
            }
        }
    }

    // Only keep capabilities that were present in `other`.
    this.retain(|k, _| other.get(k).is_some());
}

pub(crate) fn shared_with_local(map: &KeyDynValueMap) -> KeyDynValueMap {
    let mut local = local_map().clone();
    intersect(&mut local, map);
    local
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use qi_value::KeyDynValueMap;

    #[test]
    fn intersect_map() {
        let mut m = KeyDynValueMap::from_iter([
            ("A".to_owned(), true.into_value()),
            ("B".to_owned(), true.into_value()),
            ("C".to_owned(), false.into_value()),
            ("D".to_owned(), false.into_value()),
            ("E".to_owned(), true.into_value()),
            ("F".to_owned(), false.into_value()),
        ]);
        let m2 = KeyDynValueMap::from_iter([
            ("A".to_owned(), true.into_value()),
            ("B".to_owned(), false.into_value()),
            ("C".to_owned(), true.into_value()),
            ("D".to_owned(), false.into_value()),
            ("G".to_owned(), true.into_value()),
            ("H".to_owned(), false.into_value()),
        ]);
        intersect(&mut m, &m2);
        assert_matches!(m.get_as("A"), Some(true));
        assert_matches!(m.get_as("B"), Some(false));
        assert_matches!(m.get_as("C"), Some(false));
        assert_matches!(m.get_as("D"), Some(false));
        assert_matches!(m.get("E"), None);
        assert_matches!(m.get("F"), None);
        assert_matches!(m.get("G"), None);
        assert_matches!(m.get("H"), None);
        assert_matches!(m.get("I"), None);
    }
}
