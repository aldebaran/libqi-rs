use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

trait Action:
    std::fmt::Debug
    + std::hash::Hash
    + Eq
    + Ord
    + Clone
    + Copy
    + ToPrimitive
    + FromPrimitive
    + std::convert::Into<u32>
    + std::convert::TryFrom<u32>
    + serde::Serialize
    + for<'de> serde::Deserialize<'de>
{
}

#[derive(
    Debug,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    FromPrimitive,
    ToPrimitive,
    serde::Serialize,
    serde::Deserialize,
)]
#[repr(u32)]
#[serde(try_from = "u32")]
#[serde(into = "u32")]
pub enum Server {
    Connect = 4,
    Authenticate = 8,
}

impl Action for Server {}

impl Default for Server {
    fn default() -> Self {
        Self::Connect
    }
}

impl std::convert::Into<u32> for Server {
    fn into(self) -> u32 {
        self.to_u32().unwrap()
    }
}

impl std::convert::TryFrom<u32> for Server {
    type Error = ServerError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_u32(value).ok_or(ServerError(value))
    }
}

#[derive(Debug, thiserror::Error, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[error("invalid server action value {0}")]
pub struct ServerError(u32);

#[derive(
    Debug,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    FromPrimitive,
    ToPrimitive,
    serde::Serialize,
    serde::Deserialize,
)]
#[repr(u32)]
#[serde(try_from = "u32")]
#[serde(into = "u32")]
pub enum ServiceDirectory {
    Service = 100,
    Services = 101,
    RegisterService = 102,
    UnregisterService = 103,
    ServiceReady = 104,
    UpdateServiceInfo = 105,
    ServiceAdded = 106,
    ServiceRemoved = 107,
    MachineId = 108,
}

impl Action for ServiceDirectory {}

impl Default for ServiceDirectory {
    fn default() -> Self {
        Self::Service
    }
}

impl std::convert::Into<u32> for ServiceDirectory {
    fn into(self) -> u32 {
        self.to_u32().unwrap()
    }
}

impl std::convert::TryFrom<u32> for ServiceDirectory {
    type Error = ServiceDirectoryError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_u32(value).ok_or(ServiceDirectoryError(value))
    }
}

#[derive(Debug, thiserror::Error, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[error("invalid service directory action value {0}")]
pub struct ServiceDirectoryError(u32);

#[derive(
    Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, serde::Serialize, serde::Deserialize,
)]
#[repr(u32)]
#[serde(try_from = "u32")]
#[serde(into = "u32")]
pub enum BoundObject {
    RegisterEvent,
    UnregisterEvent,
    Metaobject,
    Terminate,
    Property,
    SetProperty,
    Properties,
    RegisterEventWithSignature,
    BoundFunction(u32),
}

impl BoundObject {
    const ID_REGISTER_EVENT: u32 = 0;
    const ID_UNREGISTER_EVENT: u32 = 1;
    const ID_METAOBJECT: u32 = 2;
    const ID_TERMINATE: u32 = 3;
    const ID_PROPERTY: u32 = 5; // not a typo, there is no action 4
    const ID_SET_PROPERTY: u32 = 6;
    const ID_PROPERTIES: u32 = 7;
    const ID_REGISTER_EVENT_WITH_SIGNATURE: u32 = 8;
}

impl Action for BoundObject {}

impl Default for BoundObject {
    fn default() -> Self {
        Self::RegisterEvent
    }
}

impl FromPrimitive for BoundObject {
    fn from_u32(n: u32) -> Option<Self> {
        Some(match n {
            Self::ID_REGISTER_EVENT => Self::RegisterEvent,
            Self::ID_UNREGISTER_EVENT => Self::UnregisterEvent,
            Self::ID_METAOBJECT => Self::Metaobject,
            Self::ID_TERMINATE => Self::Terminate,
            Self::ID_PROPERTY => Self::Property,
            Self::ID_SET_PROPERTY => Self::SetProperty,
            Self::ID_PROPERTIES => Self::Properties,
            Self::ID_REGISTER_EVENT_WITH_SIGNATURE => Self::RegisterEventWithSignature,
            _ => Self::BoundFunction(n),
        })
    }

    fn from_i64(n: i64) -> Option<Self> {
        Self::from_u32(n.try_into().ok()?)
    }

    fn from_u64(n: u64) -> Option<Self> {
        Self::from_u32(n.try_into().ok()?)
    }
}

impl ToPrimitive for BoundObject {
    fn to_u32(&self) -> Option<u32> {
        Some(match self {
            Self::RegisterEvent => Self::ID_REGISTER_EVENT,
            Self::UnregisterEvent => Self::ID_UNREGISTER_EVENT,
            Self::Metaobject => Self::ID_METAOBJECT,
            Self::Terminate => Self::ID_TERMINATE,
            Self::Property => Self::ID_PROPERTY,
            Self::SetProperty => Self::ID_SET_PROPERTY,
            Self::Properties => Self::ID_PROPERTIES,
            Self::RegisterEventWithSignature => Self::ID_REGISTER_EVENT_WITH_SIGNATURE,
            Self::BoundFunction(n) => *n,
        })
    }

    fn to_i64(&self) -> Option<i64> {
        Some(self.to_u32().unwrap().into())
    }

    fn to_u64(&self) -> Option<u64> {
        Some(self.to_u32().unwrap().into())
    }
}

impl std::convert::Into<u32> for BoundObject {
    fn into(self) -> u32 {
        self.to_u32().unwrap()
    }
}

impl std::convert::TryFrom<u32> for BoundObject {
    type Error = BoundObjectError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_u32(value).ok_or(BoundObjectError(value))
    }
}

#[derive(Debug, thiserror::Error, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[error("invalid bound object action value {0}")]
pub struct BoundObjectError(u32);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    pub fn test_server_ser_de() {
        assert_tokens(&Server::Authenticate, &[Token::U32(8)]);
    }

    #[test]
    pub fn test_service_directory_ser_de() {
        assert_tokens(&ServiceDirectory::UnregisterService, &[Token::U32(103)]);
    }

    #[test]
    pub fn test_bound_object_ser_de() {
        assert_tokens(&BoundObject::Terminate, &[Token::U32(3)]);
        assert_tokens(&BoundObject::BoundFunction(3920), &[Token::U32(3920)]);
    }
}
