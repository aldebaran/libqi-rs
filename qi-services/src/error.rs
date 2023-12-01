use crate::session::{authentication, capabilities};
use qi_format as format;
use qi_messaging::{self as messaging, message};
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

    #[error("missing required capability")]
    MissingRequiredCapability(#[from] capabilities::ExpectedKeyValueError<bool>),

    #[error("the call request has been canceled")]
    Canceled,

    #[error("format error")]
    Format(#[from] format::Error),

    #[error("method not found {0}")]
    MethodNotFound(object::MemberAddress),

    #[error("property not found {0}")]
    PropertyNotFound(object::MemberAddress),

    #[error("signal not found {0}")]
    SignalNotFound(object::MemberAddress),

    #[error("value conversion error")]
    FromValue(#[from] FromValueError),

    #[error("no available handler for message address {0}")]
    NoHandler(message::Address),

    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<messaging::Error> for Error {
    fn from(err: messaging::Error) -> Self {
        match err {
            messaging::Error::Canceled => Self::Canceled,
            messaging::Error::Disconnected => Self::Disconnected,
            messaging::Error::Message(err) => Self::Other(err.into()),
            messaging::Error::Other(err) => Self::Other(err),
            messaging::Error::Format(err) => Self::Format(err),
        }
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Disconnected
    }
}

impl From<Error> for messaging::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Canceled => Self::Canceled,
            Error::Disconnected => Self::Disconnected,
            Error::Other(err) => Self::Other(err),
            Error::Format(err) => Self::Format(err),
            _ => Self::Other(err.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("message decoding error")]
    Decode(#[from] messaging::codec::DecodeError),

    #[error("message encoding error")]
    Encode(#[from] messaging::codec::EncodeError),

    #[error("IO error")]
    Io(#[from] std::io::Error),
}
