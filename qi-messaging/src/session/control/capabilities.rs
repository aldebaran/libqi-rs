pub(in crate::session) use crate::capabilities::CapabilitiesMap;
use once_cell::sync::OnceCell;

#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct Base {
    client_server_socket: bool,
    meta_object_cache: bool,
    message_flags: bool,
    remote_cancelable_calls: bool,
    object_ptr_uid: bool,
    relative_endpoint_uri: bool,
}

impl Base {
    const CLIENT_SERVER_SOCKET: &'static str = "ClientServerSocket";
    const META_OBJECT_CACHE: &'static str = "MetaObjectCache";
    const MESSAGE_FLAGS: &'static str = "MessageFlags";
    const REMOTE_CANCELABLE_CALLS: &'static str = "RemoteCancelableCalls";
    const OBJECT_PTR_UID: &'static str = "ObjectPtrUID";
    const RELATIVE_ENDPOINT_URI: &'static str = "RelativeEndpointURI";

    fn from_capabilities(map: &CapabilitiesMap) -> Self {
        Self {
            client_server_socket: map.has_flag_capability(Self::CLIENT_SERVER_SOCKET),
            meta_object_cache: map.has_flag_capability(Self::META_OBJECT_CACHE),
            message_flags: map.has_flag_capability(Self::MESSAGE_FLAGS),
            remote_cancelable_calls: map.has_flag_capability(Self::REMOTE_CANCELABLE_CALLS),
            object_ptr_uid: map.has_flag_capability(Self::OBJECT_PTR_UID),
            relative_endpoint_uri: map.has_flag_capability(Self::RELATIVE_ENDPOINT_URI),
        }
    }

    fn to_capabilities(self) -> CapabilitiesMap {
        CapabilitiesMap::from_iter([
            (Self::CLIENT_SERVER_SOCKET, self.client_server_socket),
            (Self::META_OBJECT_CACHE, self.meta_object_cache),
            (Self::MESSAGE_FLAGS, self.message_flags),
            (Self::REMOTE_CANCELABLE_CALLS, self.remote_cancelable_calls),
            (Self::OBJECT_PTR_UID, self.object_ptr_uid),
            (Self::RELATIVE_ENDPOINT_URI, self.relative_endpoint_uri),
        ])
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
pub struct ExpectedKeyValueError<T>(String, T);

impl CapabilitiesMapExt for CapabilitiesMap {
    /// Checks that the capabilities have the required values that are only supported by this implementation.
    ///
    /// This implementation does not yet handle all the possible effects of each capability cases. This function
    /// ensures that the capabilities have the only values that are handle at the moment.
    fn check_required(&self) -> Result<&Self, ExpectedKeyValueError<bool>> {
        let base = Base::from_capabilities(self);

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

    fn check_intersect_with_local(mut self) -> Result<Self, ExpectedKeyValueError<bool>> {
        self.intersect(local()).check_required()?;
        Ok(self)
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

static LOCAL_CAPABILITIES: OnceCell<CapabilitiesMap> = OnceCell::new();

pub(super) fn local() -> &'static CapabilitiesMap {
    LOCAL_CAPABILITIES.get_or_init(|| LOCAL_BASE_CAPABILITIES.to_capabilities())
}
