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

// /// A type erased error.
// pub struct AnyError(Box<dyn std::error::Error + Send + Sync>);

// impl AnyError {
//     pub fn new<T>(err: T) -> Self
//     where
//         T: Into<Box<dyn std::error::Error + Send + Sync>>,
//     {
//         Self(err.into())
//     }

//     pub fn as_dyn(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
//         &*self.0
//     }

//     pub fn as_mut_dyn(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
//         &mut *self.0
//     }

//     pub fn downcast<T>(self) -> Result<Box<T>, AnyError>
//     where
//         T: std::error::Error + 'static,
//     {
//         self.0.downcast().map_err(Self)
//     }
// }

// impl std::ops::Deref for AnyError {
//     type Target = dyn std::error::Error + Send + Sync + 'static;

//     fn deref(&self) -> &Self::Target {
//         self.as_dyn()
//     }
// }

// impl std::ops::DerefMut for AnyError {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.as_mut_dyn()
//     }
// }

// impl From<Box<dyn std::error::Error + Send + Sync>> for AnyError {
//     fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
//         Self(value)
//     }
// }

// impl std::fmt::Debug for AnyError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.0.fmt(f)
//     }
// }

// impl std::fmt::Display for AnyError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.0.fmt(f)
//     }
// }

// impl std::error::Error for AnyError {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         self.0.source()
//     }
// }
