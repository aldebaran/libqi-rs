use crate::{
    format,
    message::{self, Flags, Id, Message, Payload, Type},
    types::{self, Dynamic},
};
use derive_more::{From, Into};
use std::cmp::Ordering;

const SERVICE: message::Service = message::Service::new(0);

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub(crate) struct AdvertiseCapabilities {
    id: Id,
    map: Map,
}

impl AdvertiseCapabilities {
    pub fn from_message(msg: Message) -> Result<Self, FromMessageError> {
        if msg.ty != Type::Capabilities {
            return Err(FromMessageError::BadType(msg));
        }
        Ok(Self {
            id: msg.id,
            map: format::from_bytes(msg.payload.as_ref())?,
        })
    }

    pub fn into_message(self) -> Result<Message, IntoMessageError> {
        use format::to_bytes;
        Ok(Message {
            id: self.id,
            ty: Type::Capabilities,
            flags: Flags::empty(),
            service: SERVICE,
            payload: Payload::new(to_bytes(&self.map)?),
            ..Default::default()
        })
    }
}

#[derive(thiserror::Error, Debug)]
#[error("payload format error: {0}")]
pub struct IntoMessageError(#[from] format::Error);

#[derive(thiserror::Error, Debug)]
pub enum FromMessageError {
    #[error("message {0} has not the \"capabilities\" type")]
    BadType(Message),

    #[error("payload format error: {0}")]
    PayloadFormatError(#[from] format::Error),
}

impl TryFrom<AdvertiseCapabilities> for Message {
    type Error = IntoMessageError;
    fn try_from(c: AdvertiseCapabilities) -> Result<Self, Self::Error> {
        c.into_message()
    }
}

impl TryFrom<Message> for AdvertiseCapabilities {
    type Error = FromMessageError;
    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        Self::from_message(msg)
    }
}

type MapImpl = types::Map<String, Dynamic>;

#[derive(
    Default, Clone, PartialEq, Eq, Debug, From, Into, serde::Serialize, serde::Deserialize,
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

    pub fn resolve_minimums_against<F>(&mut self, other: &Self, mut default: F)
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
                    default(&mut value);
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
            default(value);
        }
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

pub fn default_capability(value: &mut Dynamic) {
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

#[derive(derive_new::new, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct Common {
    #[new(value = "true")]
    pub client_server_socket: bool,
    #[new(value = "true")]
    pub message_flags: bool,
    #[new(value = "true")]
    pub remote_cancelable_calls: bool,
    #[new(value = "true")]
    pub object_ptr_uid: bool,
    #[new(value = "true")]
    pub relative_endpoint_uri: bool,
}

impl Common {
    pub const CLIENT_SERVER_SOCKET: &'static str = "ClientServerSocket";
    pub const MESSAGE_FLAGS: &'static str = "MessageFlags";
    pub const REMOTE_CANCELABLE_CALLS: &'static str = "RemoteCancelableCalls";
    pub const OBJECT_PTR_UID: &'static str = "ObjectPtrUID";
    pub const RELATIVE_ENDPOINT_URI: &'static str = "RelativeEndpointURI";

    pub fn from_map(map: Map) -> Self {
        Self {
            client_server_socket: map.has_flag_capability(Self::CLIENT_SERVER_SOCKET),
            message_flags: map.has_flag_capability(Self::MESSAGE_FLAGS),
            remote_cancelable_calls: map.has_flag_capability(Self::REMOTE_CANCELABLE_CALLS),
            object_ptr_uid: map.has_flag_capability(Self::OBJECT_PTR_UID),
            relative_endpoint_uri: map.has_flag_capability(Self::RELATIVE_ENDPOINT_URI),
        }
    }

    pub fn into_map(self) -> Map {
        Map::from_iter([
            (Self::CLIENT_SERVER_SOCKET, self.client_server_socket),
            (Self::MESSAGE_FLAGS, self.message_flags),
            (Self::REMOTE_CANCELABLE_CALLS, self.remote_cancelable_calls),
            (Self::OBJECT_PTR_UID, self.object_ptr_uid),
            (Self::RELATIVE_ENDPOINT_URI, self.relative_endpoint_uri),
        ])
    }
}

impl Default for Common {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Map> for Common {
    fn from(map: Map) -> Self {
        Self::from_map(map)
    }
}

impl From<Common> for Map {
    fn from(common: Common) -> Self {
        common.into_map()
    }
}

pub fn local() -> Map {
    Common::new().into_map()
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
        m.resolve_minimums_against(&m2, default_capability);
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
