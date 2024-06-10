use crate::session;
use qi_format as format;
use qi_messaging as messaging;
use qi_value::{object, FromValueError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ParseUrl(#[from] url::ParseError),

    #[error("authentication error")]
    Authentication(#[from] session::authentication::Error),

    #[error("no reachable endpoint")]
    NoReachableEndpoint,

    #[error("no available message handler for address {0}")]
    NoMessageHandler(messaging::message::Address),

    #[error("method not found {0}")]
    MethodNotFound(object::MemberAddress),

    #[error("property not found {0}")]
    PropertyNotFound(object::MemberAddress),

    #[error("signal not found {0}")]
    SignalNotFound(object::MemberAddress),

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
