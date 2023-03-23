use crate::message::{Flags, Message, Payload, Type};
pub use crate::{
    format,
    message::{Action, Id, Object, Service},
    types::Dynamic,
};

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Call<T> {
    id: Id,
    dynamic_payload: bool,
    return_type: bool,
    service: Service,
    object: Object,
    action: Action,
    argument: T,
}

impl<T> Call<T> {
    pub fn id(&self) -> Id {
        self.id
    }

    fn flags(&self) -> Flags {
        let mut flags = Flags::empty();
        flags.set(Flags::DYNAMIC_PAYLOAD, self.dynamic_payload);
        flags.set(Flags::RETURN_TYPE, self.return_type);
        flags
    }

    pub fn from_message(msg: Message) -> Result<Self, CallFromMessageError>
    where
        T: serde::de::DeserializeOwned,
    {
        Ok(Self {
            id: msg.id,
            dynamic_payload: msg.flags.has_dynamic_payload(),
            return_type: msg.flags.has_return_type(),
            service: msg.service,
            object: msg.object,
            action: msg.action,
            argument: format::from_bytes(msg.payload.bytes())?,
        })
    }

    pub fn into_message(self) -> Result<Message, CallIntoMessageError>
    where
        T: serde::Serialize,
    {
        Ok(Message {
            id: self.id,
            ty: Type::Call,
            flags: self.flags(),
            service: self.service,
            object: self.object,
            action: self.action,
            payload: Payload::new(format::to_bytes(&self.argument)?),
        })
    }
}

#[derive(thiserror::Error, Debug)]
#[error("payload format error: {0}")]
pub struct CallIntoMessageError(#[from] format::Error);

#[derive(thiserror::Error, Debug)]
pub enum CallFromMessageError {
    #[error("message {0} has not the \"{}\" type", Type::Call)]
    BadType(Message),

    #[error("payload format error: {0}")]
    PayloadFormatError(#[from] format::Error),
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct CallBuilder {
    id: Id,
    dynamic_payload: bool,
    return_type: bool,
    service: Service,
    object: Object,
    action: Action,
}

impl CallBuilder {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn dynamic_payload(mut self, value: bool) -> Self {
        self.dynamic_payload = value;
        self
    }

    pub fn return_type(mut self, value: bool) -> Self {
        self.return_type = value;
        self
    }

    pub fn service(mut self, value: Service) -> Self {
        self.service = value;
        self
    }

    pub fn object(mut self, value: Object) -> Self {
        self.object = value;
        self
    }

    pub fn action(mut self, value: Action) -> Self {
        self.action = value;
        self
    }

    pub fn argument<T>(self, argument: T) -> CallBuilderWithArg<T> {
        CallBuilderWithArg {
            call: Call {
                id: self.id,
                dynamic_payload: self.dynamic_payload,
                return_type: self.return_type,
                service: self.service,
                object: self.object,
                action: self.action,
                argument,
            },
        }
    }
}

pub struct CallBuilderWithArg<T> {
    call: Call<T>,
}

impl<T> CallBuilderWithArg<T> {
    pub fn build(self) -> Call<T> {
        self.call
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Reply<T> {
    id: Id,
    dynamic_payload: bool,
    service: Service,
    object: Object,
    action: Action,
    value: T,
}

impl<T> Reply<T> {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn has_dynamic_value(&self) -> bool {
        self.dynamic_payload
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    fn flags(&self) -> Flags {
        let mut flags = Flags::empty();
        flags.set(Flags::DYNAMIC_PAYLOAD, self.dynamic_payload);
        flags
    }

    pub fn from_message(msg: Message) -> Result<Self, ReplyFromMessageError>
    where
        T: serde::de::DeserializeOwned,
    {
        Ok(Self {
            id: msg.id,
            dynamic_payload: msg.flags.has_dynamic_payload(),
            service: msg.service,
            object: msg.object,
            action: msg.action,
            value: format::from_bytes(msg.payload.bytes())?,
        })
    }

    pub fn into_message(self) -> Result<Message, ReplyIntoMessageError>
    where
        T: serde::Serialize,
    {
        Ok(Message {
            id: self.id,
            ty: Type::Reply,
            flags: self.flags(),
            service: self.service,
            object: self.object,
            action: self.action,
            payload: Payload::new(format::to_bytes(&self.value)?),
        })
    }
}

#[derive(thiserror::Error, Debug)]
#[error("payload format error: {0}")]
pub struct ReplyIntoMessageError(#[from] format::Error);

#[derive(thiserror::Error, Debug)]
pub enum ReplyFromMessageError {
    #[error("message {0} has not the \"{}\" type", Type::Reply)]
    BadType(Message),

    #[error("payload format error: {0}")]
    PayloadFormatError(#[from] format::Error),
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct ReplyBuilder {
    id: Id,
    dynamic_payload: bool,
    service: Service,
    object: Object,
    action: Action,
}

impl ReplyBuilder {
    pub fn new_for<T>(call: &Call<T>) -> Self {
        Self {
            id: call.id,
            dynamic_payload: false,
            service: call.service,
            object: call.object,
            action: call.action,
        }
    }

    pub fn dynamic_payload(mut self, value: bool) -> Self {
        self.dynamic_payload = value;
        self
    }

    pub fn value<T>(self, value: T) -> ReplyBuilderWithValue<T> {
        ReplyBuilderWithValue {
            reply: Reply {
                id: self.id,
                dynamic_payload: self.dynamic_payload,
                service: self.service,
                object: self.object,
                action: self.action,
                value,
            },
        }
    }
}

pub struct ReplyBuilderWithValue<T> {
    reply: Reply<T>,
}

impl<T> ReplyBuilderWithValue<T> {
    pub fn build(self) -> Reply<T> {
        self.reply
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Error {
    id: Id,
    service: Service,
    object: Object,
    action: Action,
    description: String,
}

impl Error {
    pub fn from_message(message: Message) -> Result<Self, ErrorFromMessageError> {
        Ok(Self {
            id: message.id,
            service: message.service,
            object: message.object,
            action: message.action,
            description: {
                let description: Dynamic = format::from_bytes(message.payload.as_ref())?;
                description
                    .into_string()
                    .ok_or(ErrorFromMessageError::DynamicValueIsNotAString)?
            },
        })
    }

    pub fn into_message(self) -> Result<Message, ErrorIntoMessageError> {
        Ok(Message {
            id: self.id,
            ty: Type::Error,
            flags: Flags::empty(),
            service: self.service,
            object: self.object,
            action: self.action,
            payload: Payload::new(format::to_bytes(&Dynamic::from(self.description))?),
        })
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn service(&self) -> Service {
        self.service
    }

    pub fn object(&self) -> Object {
        self.object
    }

    pub fn action(&self) -> Action {
        self.action
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn into_description(self) -> String {
        self.description
    }
}

#[derive(thiserror::Error, Debug)]
#[error("payload format error: {0}")]
pub struct ErrorIntoMessageError(#[from] format::Error);

#[derive(thiserror::Error, Debug)]
pub enum ErrorFromMessageError {
    #[error("message {0} has not the \"{}\" type", Type::Error)]
    BadType(Message),

    #[error("message value is a dynamic value but it does not contain a string")]
    DynamicValueIsNotAString,

    #[error("payload format error: {0}")]
    PayloadFormatError(#[from] format::Error),
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct ErrorBuilder {
    error: Error,
}

impl ErrorBuilder {
    pub fn new_for<T>(call: &Call<T>) -> Self {
        Self {
            error: Error {
                id: call.id,
                service: call.service,
                object: call.object,
                action: call.action,
                description: String::new(),
            },
        }
    }

    pub fn message(mut self, message: String) -> Self {
        self.error.description = message;
        self
    }

    pub fn build(self) -> Error {
        self.error
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Canceled {
    id: Id,
    service: Service,
    object: Object,
    action: Action,
}

impl Canceled {
    fn new_for<T>(call: &Call<T>) -> Self {
        Self {
            id: call.id,
            service: call.service,
            object: call.object,
            action: call.action,
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, derive_more::From)]
pub enum Response<T> {
    Reply(Reply<T>),
    Error(Error),
    Canceled(Canceled),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_from_message() {
        let msg = Message {
            id: Id::new(990340),
            ty: Type::Error,
            flags: Flags::empty(),
            service: Service::new(47),
            object: Object::new(1),
            action: Action::new(178),
            payload: Payload::new(vec![
                0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
                0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
                0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
            ]),
        };
        let error = Error::from_message(msg).unwrap();
        assert_eq!(
            error,
            Error {
                id: Id::new(990340),
                service: Service::new(47),
                object: Object::new(1),
                action: Action::new(178),
                description: "The robot is not localized".to_owned(),
            }
        );
    }
}
