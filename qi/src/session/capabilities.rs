use once_cell::sync::Lazy;
use qi_messaging::CapabilitiesMap;
use qi_value::{Dynamic, IntoValue};
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

    fn from_map(map: &CapabilitiesMap) -> Self {
        Self {
            remote_cancelable_calls: map
                .get(Self::REMOTE_CANCELABLE_CALLS)
                .cloned()
                .and_then(|Dynamic(v)| v.cast_into().ok())
                .unwrap_or(false),
            object_ptr_uid: map
                .get(Self::OBJECT_PTR_UID)
                .cloned()
                .and_then(|Dynamic(v)| v.cast_into().ok())
                .unwrap_or(false),
            relative_endpoint_uri: map
                .get(Self::RELATIVE_ENDPOINT_URI)
                .cloned()
                .and_then(|Dynamic(v)| v.cast_into().ok())
                .unwrap_or(false),
        }
    }

    fn to_map(self) -> CapabilitiesMap<'static> {
        CapabilitiesMap::from_iter([
            (
                Self::REMOTE_CANCELABLE_CALLS.to_owned(),
                Dynamic(self.remote_cancelable_calls.into_value()),
            ),
            (
                Self::OBJECT_PTR_UID.to_owned(),
                Dynamic(self.object_ptr_uid.into_value()),
            ),
            (
                Self::RELATIVE_ENDPOINT_URI.to_owned(),
                Dynamic(self.relative_endpoint_uri.into_value()),
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
pub(crate) struct ExpectedKeyValueError<T>(String, T);

impl<T> From<ExpectedKeyValueError<T>> for crate::Error
where
    T: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    fn from(err: ExpectedKeyValueError<T>) -> Self {
        Self::Other(err.into())
    }
}

/// Checks that the capabilities have the required values that are only supported by this implementation.
///
/// This implementation does not yet handle all the possible effects of each capability cases. This function
/// ensures that the capabilities have the only values that are handle at the moment.
pub(crate) fn check_required(
    map: &CapabilitiesMap,
) -> Result<Capabilities, ExpectedKeyValueError<bool>> {
    let capabilities = Capabilities::from_map(map);

    // TODO: Implement capabilities so that this function always succeeds, so that we may remove it.
    if !capabilities.remote_cancelable_calls {
        return Err(ExpectedKeyValueError(
            Capabilities::REMOTE_CANCELABLE_CALLS.into(),
            true,
        ));
    }
    if !capabilities.object_ptr_uid {
        return Err(ExpectedKeyValueError(
            Capabilities::OBJECT_PTR_UID.into(),
            true,
        ));
    }
    if !capabilities.relative_endpoint_uri {
        return Err(ExpectedKeyValueError(
            Capabilities::RELATIVE_ENDPOINT_URI.into(),
            true,
        ));
    }
    Ok(capabilities)
}

pub(crate) fn local_map() -> &'static CapabilitiesMap<'static> {
    const LOCAL_CAPABILITIES: Capabilities = Capabilities::new();
    static LOCAL_CAPABILITIES_MAP: Lazy<CapabilitiesMap> =
        Lazy::new(|| LOCAL_CAPABILITIES.to_map());
    &LOCAL_CAPABILITIES_MAP
}

fn intersect(this: &mut CapabilitiesMap, other: &CapabilitiesMap) {
    for (key, other_value) in other.iter() {
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

pub(crate) fn shared_with_local(map: &CapabilitiesMap) -> CapabilitiesMap<'static> {
    let mut local = local_map().clone();
    intersect(&mut local, map);
    local
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        messaging::CapabilitiesMap,
        value::{Dynamic, Value},
    };
    use assert_matches::assert_matches;

    #[test]
    fn intersect_map() {
        let mut m = CapabilitiesMap::from_iter([
            ("A".to_owned(), Dynamic(true.into_value())),
            ("B".to_owned(), Dynamic(true.into_value())),
            ("C".to_owned(), Dynamic(false.into_value())),
            ("D".to_owned(), Dynamic(false.into_value())),
            ("E".to_owned(), Dynamic(true.into_value())),
            ("F".to_owned(), Dynamic(false.into_value())),
        ]);
        let m2 = CapabilitiesMap::from_iter([
            ("A".to_owned(), Dynamic(true.into_value())),
            ("B".to_owned(), Dynamic(false.into_value())),
            ("C".to_owned(), Dynamic(true.into_value())),
            ("D".to_owned(), Dynamic(false.into_value())),
            ("G".to_owned(), Dynamic(true.into_value())),
            ("H".to_owned(), Dynamic(false.into_value())),
        ]);
        intersect(&mut m, &m2);
        assert_matches!(m.get("A"), Some(Dynamic(Value::Bool(true))));
        assert_matches!(m.get("B"), Some(Dynamic(Value::Bool(false))));
        assert_matches!(m.get("C"), Some(Dynamic(Value::Bool(false))));
        assert_matches!(m.get("D"), Some(Dynamic(Value::Bool(false))));
        assert_matches!(m.get("E"), None);
        assert_matches!(m.get("F"), None);
        assert_matches!(m.get("G"), None);
        assert_matches!(m.get("H"), None);
        assert_matches!(m.get("I"), None);
    }
}
