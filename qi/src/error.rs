use crate::session::authentication;
use qi_value::{object, FromValueError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported URI scheme \"{0}\"")]
    UnsupportedUriScheme(String),

    #[error("invalid URI port")]
    InvalidUriPort(#[source] std::num::ParseIntError),

    #[error(transparent)]
    ValidateUri(#[from] iri_string::validate::Error),

    #[error("authentication error")]
    Authentication(#[from] authentication::Error),

    #[error("disconnected")]
    Disconnected,

    #[error("no reachable endpoint")]
    NoReachableEndpoint,

    #[error("the call request has been canceled")]
    Canceled,

    #[error("method not found {0}")]
    MethodNotFound(object::MemberAddress),

    #[error("property not found {0}")]
    PropertyNotFound(object::MemberAddress),

    #[error("signal not found {0}")]
    SignalNotFound(object::MemberAddress),

    #[error("value conversion error")]
    FromValue(#[from] FromValueError),

    #[error("IO error")]
    Io(#[from] std::io::Error),

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

impl From<qi_messaging::Error> for Error {
    fn from(err: qi_messaging::Error) -> Self {
        match err {
            qi_messaging::Error::Canceled => Self::Canceled,
            qi_messaging::Error::Disconnected => Self::Disconnected,
            qi_messaging::Error::Message(err) => Self::Other(err.into()),
            qi_messaging::Error::Other(err) => Self::Other(err),
            qi_messaging::Error::Format(err) => Self::Other(err.into()),
        }
    }
}

impl From<qi_format::Error> for Error {
    fn from(err: qi_format::Error) -> Self {
        Self::Other(err.into())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Disconnected
    }
}

impl From<Error> for qi_messaging::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Canceled => Self::Canceled,
            Error::Disconnected => Self::Disconnected,
            Error::Other(err) => Self::Other(err),
            _ => Self::Other(err.into()),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, thiserror::Error)]
#[error("no available handler for message address {0}")]
pub(crate) struct NoMessageHandlerError(pub(crate) qi_messaging::message::Address);

impl From<NoMessageHandlerError> for qi_messaging::Error {
    fn from(err: NoMessageHandlerError) -> Self {
        Self::Other(err.into())
    }
}
