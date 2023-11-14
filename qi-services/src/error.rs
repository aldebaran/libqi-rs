use qi_format as format;
use qi_value::{ActionId, FromValueError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the call request has been canceled")]
    Canceled,

    #[error("the client is disconnected")]
    Disconnected,

    #[error("format error")]
    Format(#[from] format::Error),

    #[error("no such method error")]
    NoSuchMethod(#[from] NoSuchMethodError),

    #[error("value conversion error")]
    FromValue(#[from] FromValueError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<qi_messaging::Error> for Error {
    fn from(err: qi_messaging::Error) -> Self {
        use qi_messaging::Error;
        match err {
            Error::Canceled => Self::Canceled,
            Error::Disconnected => Self::Disconnected,
            Error::Format(_) => todo!(),
            Error::Other(msg) => Self::Other(msg.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NoSuchMethodError {
    #[error("no such method with id {0}")]
    Id(ActionId),
    #[error("no such method {0}")]
    Name(String),
}

#[derive(Debug, thiserror::Error)]
pub enum NoSuchPropertyError {
    #[error("no such property with id {0}")]
    Id(ActionId),

    #[error("no such property {0}")]
    Name(String),
}

#[derive(Debug, thiserror::Error)]
pub enum NoSuchSignalError {
    #[error("no such signal with id {0}")]
    Id(ActionId),

    #[error("no such signal {0}")]
    Name(String),
}
