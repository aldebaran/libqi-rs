use crate::{capabilities::CapabilitiesMap, format, value::Dynamic};
use bytes::Bytes;

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
pub struct Id(pub(crate) u32);

impl Id {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
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
pub struct Version(pub(crate) u16);

impl Version {
    const CURRENT: Self = Self(0);

    pub const fn current() -> Self {
        Self::CURRENT
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::CURRENT
    }
}

#[derive(
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

impl Default for Type {
    fn default() -> Self {
        Self::Call
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
    Hash,
    Debug,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[display(fmt = "({service}, {object}, {action})")]
pub struct Address {
    pub(crate) service: Service,
    pub(crate) object: Object,
    pub(crate) action: Action,
}

impl Address {
    pub const fn service(&self) -> Service {
        self.service
    }

    pub const fn object(&self) -> Object {
        self.object
    }

    pub const fn action(&self) -> Action {
        self.action
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
    Hash,
    Debug,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Service(pub(crate) u32);

impl Service {
    pub const fn new(id: u32) -> Self {
        Self(id)
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
    Hash,
    Debug,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Object(pub(crate) u32);

impl Object {
    pub const fn new(id: u32) -> Self {
        Self(id)
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
    Hash,
    Debug,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Action(pub(crate) u32);

impl Action {
    pub const fn new(id: u32) -> Self {
        Self(id)
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
#[display(fmt = "header(id={id}, {ty}, version={version}, address={address})")]
pub struct Header {
    pub(crate) id: Id,
    pub(crate) ty: Type,
    pub(crate) body_size: usize,
    pub(crate) version: Version,
    pub(crate) address: Address,
}

#[derive(
    Default,
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
#[display(
    fmt = "message({ty}, id={id}, address={address}, body=[len={}])",
    "body.len()"
)]
pub struct Message {
    pub(crate) id: Id,
    pub(crate) ty: Type,
    pub(crate) version: Version,
    pub(crate) address: Address,
    pub(crate) body: Bytes,
}

impl Message {
    pub fn new(header: Header, body: Bytes) -> Self {
        Self {
            id: header.id,
            ty: header.ty,
            version: header.version,
            address: header.address,
            body,
        }
    }

    /// Builds a "call" message.
    ///
    /// This sets the ty, the id and the address of the message.
    pub fn call(id: Id, address: Address) -> Builder {
        Builder::new()
            .set_id(id)
            .set_ty(Type::Call)
            .set_address(address)
    }

    /// Builds a "reply" message.
    ///
    /// This sets the ty, the id and the address of the message.
    pub fn reply(id: Id, address: Address) -> Builder {
        Builder::new()
            .set_id(id)
            .set_ty(Type::Reply)
            .set_address(address)
    }

    /// Builds a "error" message.
    ///
    /// This sets the ty, the id, the address and the body of the message.
    pub fn error(id: Id, address: Address, description: &str) -> Result<Builder, format::Error> {
        Builder::new()
            .set_id(id)
            .set_ty(Type::Error)
            .set_address(address)
            .set_error_description(description)
    }

    /// Builds a "post" message.
    ///
    /// This sets the ty, the id and the address of the message.
    pub fn post(id: Id, address: Address) -> Builder {
        Builder::new()
            .set_id(id)
            .set_ty(Type::Post)
            .set_address(address)
    }

    /// Builds a "event" message.
    ///
    /// This sets the ty, the id and the address of the message.
    pub fn event(id: Id, address: Address) -> Builder {
        Builder::new()
            .set_id(id)
            .set_ty(Type::Event)
            .set_address(address)
    }

    /// Builds a "capabilities" message.
    ///
    /// This sets the ty, the id, the address and the body of the message.
    pub fn capabilities(
        id: Id,
        address: Address,
        map: &CapabilitiesMap,
    ) -> Result<Builder, format::Error> {
        Builder::new()
            .set_id(id)
            .set_ty(Type::Capabilities)
            .set_address(address)
            .set_value(&map)
    }

    /// Builds a "cancel" message.
    ///
    /// This sets the ty, the id, the address and the body of the message.
    pub fn cancel(id: Id, address: Address, call_id: Id) -> Builder {
        Builder::new()
            .set_id(id)
            .set_ty(Type::Cancel)
            .set_address(address)
            .set_value(&call_id)
            .expect("failed to serialize a message ID in the format")
    }

    /// Builds a "canceled" message.
    ///
    /// This sets the ty, the id and the address of the message.
    pub fn canceled(id: Id, address: Address) -> Builder {
        Builder::new()
            .set_id(id)
            .set_address(address)
            .set_ty(Type::Canceled)
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn ty(&self) -> Type {
        self.ty
    }

    pub fn body_size(&self) -> usize {
        self.body.len()
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub fn header(&self) -> Header {
        Header {
            id: self.id,
            ty: self.ty,
            version: self.version,
            body_size: self.body_size(),
            address: self.address,
        }
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn deserialize_body<T>(&self) -> Result<T, format::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        format::from_bytes(&self.body)
    }

    pub fn deserialize_error_description(&self) -> Result<String, format::Error> {
        let Dynamic(description) = self.deserialize_body()?;
        Ok(description)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Builder(Message);

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub(crate) fn new() -> Self {
        Self(Message::default())
    }

    fn set_id(mut self, value: Id) -> Self {
        self.0.id = value;
        self
    }

    fn set_ty(mut self, value: Type) -> Self {
        self.0.ty = value;
        self
    }

    fn set_address(mut self, value: Address) -> Self {
        self.0.address = value;
        self
    }

    pub fn set_body(mut self, body: Bytes) -> Self {
        self.0.body = body;
        self
    }

    /// Sets the serialized representation of the value in the format as the body of the message.
    /// It checks if the "dynamic payload" flag is set on the message to know how to serialize the value.
    /// If the flag is set after calling this value, the value will not be serialized coherently with the flag.
    pub fn set_value<T>(mut self, value: &T) -> Result<Self, format::Error>
    where
        T: serde::Serialize,
    {
        self.0.body = format::to_bytes(value)?;
        Ok(self)
    }

    pub fn set_error_description(self, description: &str) -> Result<Self, format::Error> {
        self.set_value(&Dynamic(description))
    }

    pub fn build(self) -> Message {
        self.0
    }
}
