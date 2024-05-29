#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("messaging endpoint has been closed")]
    EndpointClosed,

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error("operation has been canceled")]
    Canceled,

    //------------------------------------------------------------------------
    // URL related errors.
    //------------------------------------------------------------------------
    #[error("unsupported URL scheme \"{0}\"")]
    UnsupportedUrlScheme(String),

    #[error("invalid URL host: {0}")]
    InvalidUrlHost(String),

    // #[error("invalid URL port: {0}")]
    // InvalidUrlPort(String),

    //------------------------------------------------------------------------
    // Other
    //------------------------------------------------------------------------
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    pub fn other<E>(err: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::Other(err.into())
    }

    pub fn is_disconnection(&self) -> bool {
        matches!(
            self,
            Self::IO(io_err)
                if matches!(
                    io_err.kind(),
                    std::io::ErrorKind::PermissionDenied
                        | std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::NotConnected
                        | std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::TimedOut))
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::EndpointClosed
    }
}

impl<T> From<tokio_util::sync::PollSendError<T>> for Error {
    fn from(_err: tokio_util::sync::PollSendError<T>) -> Self {
        Self::EndpointClosed
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(_err: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::EndpointClosed
    }
}

impl std::str::FromStr for Error {
    type Err = std::convert::Infallible;

    fn from_str(error: &str) -> Result<Self, Self::Err> {
        Ok(Self::Other(error.into()))
    }
}
