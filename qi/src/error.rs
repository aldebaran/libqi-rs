use crate::{
    messaging::{self, message},
    value,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the call request has been canceled")]
    CallCanceled,

    #[error("there is no object method with identifier {0}")]
    MethodNotFound(value::object::MemberIdent),

    #[error(transparent)]
    Other(#[from] BoxError),
}

impl From<messaging::Error> for Error {
    fn from(err: messaging::Error) -> Self {
        match err {
            messaging::Error::LinkLost(error) => Self::Other(error),
            messaging::Error::CallError(error) => Self::Other(error.into()),
            messaging::Error::CallCanceled => Self::CallCanceled,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Other(err.into())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValueConversionError {
    #[error("the conversion of the object method return value has failed")]
    MethodReturnValue(#[source] value::FromValueError),

    #[error("the conversion of the request arguments has failed")]
    Arguments(#[source] value::FromValueError),
}

impl From<ValueConversionError> for Error {
    fn from(err: ValueConversionError) -> Self {
        Error::Other(err.into())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FormatError<E> {
    #[error("the serialization of the request arguments has failed")]
    ArgumentsSerialization(#[source] E),

    #[error("the deserialization of the request arguments has failed")]
    ArgumentsDeserialization(#[source] E),

    #[error("the serialization of the method return value has failed")]
    MethodReturnValueSerialization(#[source] E),

    #[error("the deserialization of the method return value has failed")]
    MethodReturnValueDeserialization(#[source] E),
}

impl<E> From<FormatError<E>> for crate::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: FormatError<E>) -> Self {
        Error::Other(err.into())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("there is no handler for message of type {0} to address {1}")]
pub(crate) struct NoHandlerError(pub(crate) message::Type, pub(crate) message::Address);

impl From<NoHandlerError> for Error {
    fn from(err: NoHandlerError) -> Self {
        Self::Other(err.into())
    }
}

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("the call request has been canceled")]
    CallCanceled,

    #[error("{message}")]
    Custom { message: String, is_fatal: bool },
}

impl HandlerError {
    pub(crate) fn non_fatal<E>(err: E) -> Self
    where
        E: std::string::ToString,
    {
        Self::Custom {
            message: err.to_string(),
            is_fatal: false,
        }
    }

    pub(crate) fn fatal<E>(err: E) -> Self
    where
        E: std::string::ToString,
    {
        Self::Custom {
            message: err.to_string(),
            is_fatal: true,
        }
    }
}

impl messaging::handler::Error for HandlerError {
    fn is_canceled(&self) -> bool {
        matches!(self, Self::CallCanceled)
    }

    fn is_fatal(&self) -> bool {
        matches!(self, Self::Custom { is_fatal: true, .. })
    }
}
