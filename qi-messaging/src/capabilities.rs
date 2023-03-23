use crate::{
    format,
    message::{self, Flags, Id, Message, Payload, Type},
    types,
};
use derive_more::{From, Into};

const SERVICE: message::Service = message::Service::new(0);

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub(crate) struct AdvertiseCapabilities {
    id: Id,
    map: Map,
}

impl AdvertiseCapabilities {
    pub fn new(id: Id, map: Map) -> Self {
        Self { id, map }
    }

    pub fn from_message(msg: Message) -> Result<Self, FromMessageError> {
        use format::from_bytes;
        match msg.ty {
            Type::Capabilities => Ok(Self {
                id: msg.id,
                map: from_bytes(msg.payload.as_ref())?,
            }),
            _ => Err(FromMessageError::BadType(msg)),
        }
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

    pub fn into_map(self) -> Map {
        self.map
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

type MapImpl = types::Map<String, types::Dynamic>;

#[derive(
    Default, Clone, PartialEq, Eq, Debug, From, Into, serde::Serialize, serde::Deserialize,
)]
pub struct Map(MapImpl);

impl Map {
    pub fn new() -> Self {
        Self(MapImpl::new())
    }

    pub fn set_capability(&mut self, name: &str, value: bool) -> &mut Self {
        self.0.insert(name.into(), types::Dynamic::from(value));
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
        let mut res = Map::new();
        for (capability, enabled) in self.iter() {
            res.set_capability(capability, enabled && other.has_capability(capability));
        }
        for (capability, enabled) in other.iter() {
            res.set_capability(capability, enabled && self.has_capability(capability));
        }
        res
    }
}

impl<'a> std::iter::IntoIterator for &'a Map {
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

impl<K, V> std::iter::FromIterator<(K, V)> for Map
where
    K: Into<String>,
    V: Into<bool>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(MapImpl::from_iter(
            iter.into_iter()
                .map(|(k, v)| (k.into(), types::Dynamic::from(v.into()))),
        ))
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
            client_server_socket: map.has_capability(Self::CLIENT_SERVER_SOCKET),
            message_flags: map.has_capability(Self::MESSAGE_FLAGS),
            remote_cancelable_calls: map.has_capability(Self::REMOTE_CANCELABLE_CALLS),
            object_ptr_uid: map.has_capability(Self::OBJECT_PTR_UID),
            relative_endpoint_uri: map.has_capability(Self::RELATIVE_ENDPOINT_URI),
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

    #[test]
    fn test_capability_map_merge_with() {
        let m1 = Map::from_iter([
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
        let m = m1.merged_with(&m2);
        assert!(m.has_capability("A"));
        assert!(!m.has_capability("B"));
        assert!(!m.has_capability("C"));
        assert!(!m.has_capability("D"));
        assert!(!m.has_capability("E"));
        assert!(!m.has_capability("F"));
        assert!(!m.has_capability("G"));
        assert!(!m.has_capability("H"));
    }
}
