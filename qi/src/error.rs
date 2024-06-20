use crate::{
    format, messaging, session,
    value::{self, object, FromValueError},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ParseUrl(#[from] url::ParseError),

    #[error("authentication error")]
    Authentication(#[from] session::authentication::Error),

    #[error("no reachable endpoint")]
    NoReachableEndpoint,

    #[error("no available message handler for type {0} at address {1}")]
    NoMessageHandler(messaging::message::Type, messaging::message::Address),

    #[error("object method not found {0}")]
    ObjectMethodNotFound(object::MemberIdent),

    #[error("object property not found {0}")]
    ObjectPropertyNotFound(object::MemberIdent),

    #[error("object signal not found {0}")]
    ObjectSignalNotFound(object::MemberIdent),

    #[error("bad value signature (expected \"{expected}\" but actual is \"{actual}\")")]
    BadValueSignature {
        expected: value::Signature,
        actual: value::Signature,
    },

    #[error("a service with the name \"{0}\" already exists on this node")]
    ServiceExists(String),

    #[error("service \"{0}\" not found")]
    ServiceNotFound(String),

    #[error("value conversion error")]
    FromValue(#[from] FromValueError),

    #[error(transparent)]
    Messaging(#[from] messaging::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Self::Other(err.into())
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::Other(err.into())
    }
}

impl From<format::Error> for Error {
    fn from(err: format::Error) -> Self {
        Self::Other(err.into())
    }
}

impl From<Error> for messaging::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Messaging(err) => err,
            _ => messaging::Error::other(err),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
