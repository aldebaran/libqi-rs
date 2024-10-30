use crate::CapabilitiesMap;
use qi_value::Dynamic;
pub use qi_value::{ActionId as Action, ObjectId as Object, ServiceId as Service};

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

impl Id {
    pub const DEFAULT: Self = Self(0);
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
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Version(pub u16);

impl Version {
    pub const ZERO: Self = Self(0);
}

impl Default for Version {
    fn default() -> Self {
        Self::ZERO
    }
}

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
    #[display(fmt = "call")]
    Call,
    #[display(fmt = "reply")]
    Reply,
    #[display(fmt = "error")]
    Error,
    #[display(fmt = "post")]
    Post,
    #[display(fmt = "event")]
    Event,
    #[display(fmt = "capabilities")]
    Capabilities,
    #[display(fmt = "cancel")]
    Cancel,
    #[display(fmt = "canceled")]
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
#[display(fmt = "{{{_0}.{_1}.{_2}}}")]
pub struct Address(pub Service, pub Object, pub Action);

impl Address {
    pub const DEFAULT: Self = Self(Service::DEFAULT, Object::DEFAULT, Action::DEFAULT);

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Message<Body> {
    Call {
        id: Id,
        address: Address,
        value: Body,
    },
    Reply {
        id: Id,
        address: Address,
        value: Body,
    },
    Error {
        id: Id,
        address: Address,
        error: Dynamic<String>,
    },
    Post {
        id: Id,
        address: Address,
        value: Body,
    },
    Event {
        id: Id,
        address: Address,
        value: Body,
    },
    Capabilities {
        id: Id,
        address: Address,
        capabilities: CapabilitiesMap<'static>,
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

impl<T> Default for Message<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Call {
            id: Id::DEFAULT,
            address: Address::DEFAULT,
            value: T::default(),
        }
    }
}

impl<Body> Message<Body>
where
    Body: crate::Body,
{
    pub(crate) fn into_parts(self) -> Result<(MetaData, Body), Body::Error> {
        match self {
            Message::Call { id, address, value } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Call,
                },
                value,
            )),
            Message::Reply { id, address, value } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Reply,
                },
                value,
            )),
            Message::Error { id, address, error } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Error,
                },
                Body::serialize(&error)?,
            )),
            Message::Post { id, address, value } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Post,
                },
                value,
            )),
            Message::Event { id, address, value } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Event,
                },
                value,
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
                Body::serialize(&capabilities)?,
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
                Body::serialize(&call_id)?,
            )),
            Message::Canceled { id, address } => Ok((
                MetaData {
                    id,
                    address,
                    ty: Type::Canceled,
                },
                Body::serialize(&())?,
            )),
        }
    }

    pub(crate) fn from_parts(meta: MetaData, body: Body) -> Result<Self, Body::Error> {
        let MetaData { id, address, ty } = meta;
        let msg = match ty {
            Type::Call => Self::Call {
                id,
                address,
                value: body,
            },
            Type::Reply => Self::Reply {
                id,
                address,
                value: body,
            },
            Type::Error => Self::Error {
                id,
                address,
                error: body.deserialize().map_err(Into::into)?,
            },
            Type::Post => Self::Post {
                id,
                address,
                value: body,
            },
            Type::Event => Self::Event {
                id,
                address,
                value: body,
            },
            Type::Capabilities => Self::Capabilities {
                id,
                address,
                capabilities: body.deserialize().map_err(Into::into)?,
            },
            Type::Cancel => Self::Cancel {
                id,
                address,
                call_id: body.deserialize().map_err(Into::into)?,
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
#[display(fmt = "{id}:{ty}@{address}")]
pub struct MetaData {
    pub(crate) id: Id,
    pub(crate) address: Address,
    pub(crate) ty: Type,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Oneway<Body> {
    Post(Body),
    Event(Body),
    Capabilities(CapabilitiesMap<'static>),
}

impl<Body> Oneway<Body> {
    pub fn ty(&self) -> Type {
        match self {
            Self::Post(_) => Type::Post,
            Self::Event(_) => Type::Event,
            Self::Capabilities(_) => Type::Capabilities,
        }
    }

    pub fn try_map<F, U, E>(self, f: F) -> Result<Oneway<U>, E>
    where
        F: FnOnce(Body) -> Result<U, E>,
    {
        Ok(match self {
            Self::Post(value) => Oneway::Post(f(value)?),
            Self::Event(value) => Oneway::Event(f(value)?),
            Self::Capabilities(capabilities) => Oneway::Capabilities(capabilities),
        })
    }
}

#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, serde::Serialize, serde::Deserialize,
)]
pub(crate) enum Response<T> {
    Reply(T),
    Error(String),
    Canceled,
}
