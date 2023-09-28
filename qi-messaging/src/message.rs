use crate::{capabilities::CapabilitiesMap, format, value};
use value::Dynamic;

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

bitflags::bitflags! {
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
        serde::Deserialize
    )]
    #[display(fmt = "{:b}", "self.bits()")]
    pub struct Flags: u8 {
        const DYNAMIC_PAYLOAD = 0b00000001;
        const RETURN_TYPE = 0b00000010;
    }
}

#[derive(
    derive_new::new,
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
#[display(fmt = "header(id={id}, {ty}, version={version}, flags={flags}, address={address})")]
pub struct Header {
    pub(crate) id: Id,
    pub(crate) ty: Type,
    pub(crate) body_size: usize,
    pub(crate) version: Version,
    pub(crate) flags: Flags,
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
    fmt = "message({ty}, id={id}, version={version}, flags={flags}, address={address}, body={body})"
)]
pub struct Message {
    pub(crate) id: Id,
    pub(crate) ty: Type,
    pub(crate) version: Version,
    pub(crate) flags: Flags,
    pub(crate) address: Address,
    pub(crate) body: format::Value,
}

impl Message {
    pub fn new(header: Header, body: format::Value) -> Self {
        Self {
            id: header.id,
            ty: header.ty,
            version: header.version,
            address: header.address,
            flags: header.flags,
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

    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn body_size(&self) -> usize {
        self.body.bytes_len()
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
            flags: self.flags,
            address: self.address,
        }
    }

    pub fn body(&self) -> format::Value {
        self.body.clone()
    }

    pub fn deserialize_body<T>(&self) -> Result<T, format::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        // TODO: Check DYNAMIC_PAYLOAD flag
        self.body.to_deserializable()
    }

    pub fn deserialize_error_description(
        &self,
    ) -> Result<String, DeserializeErrorDescriptionError> {
        let dynamic: Dynamic = self.deserialize_body()?;
        match dynamic {
            Dynamic::String(s) => Ok(s),
            d => Err(DeserializeErrorDescriptionError::DynamicValueIsNotAString(
                d,
            )),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DeserializeErrorDescriptionError {
    #[error("dynamic value {0} of error description is not a string")]
    DynamicValueIsNotAString(Dynamic),

    #[error(transparent)]
    Format(#[from] format::Error),
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

    pub fn set_body(mut self, body: format::Value) -> Self {
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
        // TODO: if flags has dynamic_payload bit, serialize the value as a dynamic.
        self.0.body = format::Value::from_serializable(value)?;
        Ok(self)
    }

    pub fn set_error_description(self, description: &str) -> Result<Self, format::Error> {
        self.set_value(&Dynamic::from(description))
    }

    pub fn build(self) -> Message {
        self.0
    }
}

// #[derive(
//     Default,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
//     Ord,
//     Hash,
//     Debug,
//     serde::Serialize,
//     serde::Deserialize,
// )]
// pub struct Call {
//     pub(crate) id: Id,
//     pub(crate) address: Address,
//     pub(crate) value: format::Value,
// }

// impl From<Call> for Message {
//     fn from(call: Call) -> Self {
//         Message::call(call.id, call.address)
//             .set_body(call.value)
//             .build()
//     }
// }

// #[derive(
//     Default,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
//     Ord,
//     Hash,
//     Debug,
//     serde::Serialize,
//     serde::Deserialize,
// )]
// pub struct Post {
//     pub(crate) id: Id,
//     pub(crate) address: Address,
//     pub(crate) value: format::Value,
// }

// impl From<Post> for Message {
//     fn from(post: Post) -> Self {
//         Message::post(post.id, post.address)
//             .set_body(post.value)
//             .build()
//     }
// }

// #[derive(
//     Default,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
//     Ord,
//     Hash,
//     Debug,
//     serde::Serialize,
//     serde::Deserialize,
// )]
// pub struct Event {
//     pub(crate) id: Id,
//     pub(crate) address: Address,
//     pub(crate) value: format::Value,
// }

// impl From<Event> for Message {
//     fn from(event: Event) -> Self {
//         Message::event(event.id, event.address)
//             .set_body(event.value)
//             .build()
//     }
// }

// #[derive(
//     Default,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
//     Ord,
//     Hash,
//     Debug,
//     serde::Serialize,
//     serde::Deserialize,
// )]
// pub struct Cancel {
//     pub(crate) id: Id,
//     pub(crate) address: Address,
//     pub(crate) call_id: Id,
// }

// impl From<Cancel> for Message {
//     fn from(cancel: Cancel) -> Self {
//         Message::cancel(cancel.id, cancel.address, cancel.call_id).build()
//     }
// }

// #[derive(
//     Default, Clone, PartialEq, Eq, PartialOrd, Debug, serde::Serialize, serde::Deserialize,
// )]
// pub struct Capabilities {
//     pub(crate) id: Id,
//     pub(crate) address: Address,
//     pub(crate) capabilities: CapabilitiesMap,
// }

// impl TryFrom<Capabilities> for Message {
//     type Error = format::Error;
//     fn try_from(capabilities: Capabilities) -> Result<Self, Self::Error> {
//         Ok(Message::capabilities(
//             capabilities.id,
//             capabilities.address,
//             &capabilities.capabilities,
//         )?
//         .build())
//     }
// }

// #[derive(
//     Default,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
//     Ord,
//     Hash,
//     Debug,
//     serde::Serialize,
//     serde::Deserialize,
// )]
// pub struct Reply {
//     pub(crate) id: Id,
//     pub(crate) address: Address,
//     pub(crate) value: format::Value,
// }

// impl From<Reply> for Message {
//     fn from(reply: Reply) -> Self {
//         Message::reply(reply.id, reply.address)
//             .set_body(reply.value)
//             .build()
//     }
// }

// #[derive(
//     Default,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
//     Ord,
//     Hash,
//     Debug,
//     serde::Serialize,
//     serde::Deserialize,
// )]
// pub struct Error {
//     pub(crate) id: Id,
//     pub(crate) address: Address,
//     pub(crate) description: String,
// }

// impl TryFrom<Error> for Message {
//     type Error = format::Error;
//     fn try_from(error: Error) -> Result<Self, Self::Error> {
//         Ok(Message::error(error.id, error.address, &error.description)?.build())
//     }
// }

// #[derive(
//     Default,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
//     Ord,
//     Hash,
//     Debug,
//     serde::Serialize,
//     serde::Deserialize,
// )]
// pub struct Canceled {
//     pub(crate) id: Id,
//     pub(crate) address: Address,
// }

// impl From<Canceled> for Message {
//     fn from(canceled: Canceled) -> Self {
//         Message::canceled(canceled.id, canceled.address).build()
//     }
// }
