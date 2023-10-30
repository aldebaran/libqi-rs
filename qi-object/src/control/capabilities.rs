use once_cell::sync::OnceCell;
use qi_messaging::capabilities::CapabilitiesMap;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct Capabilities {
    client_server_socket: bool,
    remote_cancelable_calls: bool,
    object_ptr_uid: bool,
    relative_endpoint_uri: bool,
}

impl Capabilities {
    const CLIENT_SERVER_SOCKET: &'static str = "ClientServerSocket";
    const REMOTE_CANCELABLE_CALLS: &'static str = "RemoteCancelableCalls";
    const OBJECT_PTR_UID: &'static str = "ObjectPtrUID";
    const RELATIVE_ENDPOINT_URI: &'static str = "RelativeEndpointURI";

    const fn new() -> Self {
        Self {
            client_server_socket: true,
            remote_cancelable_calls: true,
            object_ptr_uid: true,
            relative_endpoint_uri: true,
        }
    }

    fn from_map(map: &CapabilitiesMap) -> Self {
        Self {
            client_server_socket: map
                .get(Self::CLIENT_SERVER_SOCKET)
                .copied()
                .unwrap_or(false),
            remote_cancelable_calls: map
                .get(Self::REMOTE_CANCELABLE_CALLS)
                .copied()
                .unwrap_or(false),
            object_ptr_uid: map.get(Self::OBJECT_PTR_UID).copied().unwrap_or(false),
            relative_endpoint_uri: map
                .get(Self::RELATIVE_ENDPOINT_URI)
                .copied()
                .unwrap_or(false),
        }
    }

    fn to_map(self) -> CapabilitiesMap {
        CapabilitiesMap::from_iter([
            (Self::CLIENT_SERVER_SOCKET, self.client_server_socket),
            (Self::REMOTE_CANCELABLE_CALLS, self.remote_cancelable_calls),
            (Self::OBJECT_PTR_UID, self.object_ptr_uid),
            (Self::RELATIVE_ENDPOINT_URI, self.relative_endpoint_uri),
        ])
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) trait CapabilitiesMapExt {
    fn check_required(&self) -> Result<&Self, ExpectedKeyValueError<bool>>;
    fn check_intersect_with_local(self) -> Result<Self, ExpectedKeyValueError<bool>>
    where
        Self: Sized;
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, thiserror::Error)]
#[error("expected key \"{0}\" to have value \"{1}\"")]
pub(crate) struct ExpectedKeyValueError<T>(String, T);

/// Checks that the capabilities have the required values that are only supported by this implementation.
///
/// This implementation does not yet handle all the possible effects of each capability cases. This function
/// ensures that the capabilities have the only values that are handle at the moment.
pub(crate) fn check_required(
    map: &CapabilitiesMap,
) -> Result<Capabilities, ExpectedKeyValueError<bool>> {
    let capabilities = Capabilities::from_map(map);

    // TODO: Implement capabilities so that this function always succeeds, so that we may remove it.
    if !capabilities.client_server_socket {
        return Err(ExpectedKeyValueError(
            Capabilities::CLIENT_SERVER_SOCKET.into(),
            true,
        ));
    }
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

const LOCAL_CAPABILITIES: Capabilities = Capabilities::new();

static LOCAL_CAPABILITIES_MAP: OnceCell<CapabilitiesMap> = OnceCell::new();

pub(crate) fn local_map() -> &'static CapabilitiesMap {
    LOCAL_CAPABILITIES_MAP.get_or_init(|| LOCAL_CAPABILITIES.to_map())
}
