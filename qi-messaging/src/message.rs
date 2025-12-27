use crate::format;
use bytes::Bytes;
use qi_value::Dynamic;
pub use qi_value::{ActionId as Action, KeyDynValueMap, ObjectId as Object, ServiceId as Service};

#[derive(
    Default,
    Debug,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    derive_more::From,
    derive_more::Into,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct Id(pub u32);

#[derive(
    Default,
    Debug,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Version(pub u16);

#[derive(
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Type {
    #[default]
    #[display("call")]
    Call,
    #[display("reply")]
    Reply,
    #[display("error")]
    Error,
    #[display("post")]
    Post,
    #[display("event")]
    Event,
    #[display("capabilities")]
    Capabilities,
    #[display("cancel")]
    Cancel,
    #[display("canceled")]
    Canceled,
}

impl Type {
    pub const DEFAULT: Self = Self::Call;
}

#[derive(
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[display("{{{_0}.{_1}.{_2}}}")]
pub struct Address(pub Service, pub Object, pub Action);

impl Address {
    pub const fn service(&self) -> Service {
        self.0
    }

    pub const fn with_service(&self, service: Service) -> Self {
        Self(service, self.1, self.2)
    }

    pub const fn object(&self) -> Object {
        self.1
    }

    pub const fn with_object(&self, object: Object) -> Self {
        Self(self.0, object, self.2)
    }

    pub const fn action(&self) -> Action {
        self.2
    }

    pub const fn with_action(&self, action: Action) -> Self {
        Self(self.0, self.1, action)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Call {
        id: Id,
        address: Address,
        payload: Bytes,
    },
    Reply {
        id: Id,
        address: Address,
        payload: Bytes,
    },
    Error {
        id: Id,
        address: Address,
        error: String,
    },
    Post {
        id: Id,
        address: Address,
        payload: Bytes,
    },
    Event {
        id: Id,
        address: Address,
        payload: Bytes,
    },
    Capabilities {
        id: Id,
        address: Address,
        capabilities: KeyDynValueMap,
    },
    Cancel {
        id: Id,
        address: Address,
        call_id: Id,
    },
    Canceled {
        id: Id,
        address: Address,
    },
}

impl Default for Message {
    fn default() -> Self {
        Self::Call {
            id: Default::default(),
            address: Default::default(),
            payload: Default::default(),
        }
    }
}

impl Message {
    pub(crate) fn into_parts(self) -> Result<(MetaData, Bytes), format::Error> {
        match self {
            Message::Call {
                id,
                address,
                payload,
            } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Call,
                },
                payload,
            )),
            Message::Reply {
                id,
                address,
                payload,
            } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Reply,
                },
                payload,
            )),
            Message::Error { id, address, error } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Error,
                },
                format::to_bytes(&Dynamic(error))?,
            )),
            Message::Post {
                id,
                address,
                payload,
            } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Post,
                },
                payload,
            )),
            Message::Event {
                id,
                address,
                payload,
            } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Event,
                },
                payload,
            )),
            Message::Capabilities {
                id,
                address,
                capabilities,
            } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Capabilities,
                },
                format::to_bytes(&capabilities)?,
            )),
            Message::Cancel {
                id,
                address,
                call_id,
            } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Cancel,
                },
                format::to_bytes(&call_id)?,
            )),
            Message::Canceled { id, address } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Canceled,
                },
                format::to_bytes(&())?,
            )),
        }
    }

    pub(crate) fn from_parts(meta: MetaData, payload: Bytes) -> Result<Self, format::Error> {
        let MetaData { id, address, ty } = meta;
        let msg = match ty {
            Type::Call => Self::Call {
                id,
                address,
                payload,
            },
            Type::Reply => Self::Reply {
                id,
                address,
                payload,
            },
            Type::Error => Self::Error {
                id,
                address,
                error: format::from_slice::<Dynamic<String>>(&payload)?.into_inner(),
            },
            Type::Post => Self::Post {
                id,
                address,
                payload,
            },
            Type::Event => Self::Event {
                id,
                address,
                payload,
            },
            Type::Capabilities => Self::Capabilities {
                id,
                address,
                capabilities: format::from_slice(&payload)?,
            },
            Type::Cancel => Self::Cancel {
                id,
                address,
                call_id: format::from_slice(&payload)?,
            },
            Type::Canceled => Self::Canceled { id, address },
        };
        Ok(msg)
    }
}

#[derive(
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[display("{id}:{ty}@{address}")]
pub struct MetaData {
    pub(crate) id: Id,
    pub(crate) address: Address,
    pub(crate) ty: Type,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub(crate) enum Response {
    Reply(Bytes),
    Error(String),
    Canceled,
}
