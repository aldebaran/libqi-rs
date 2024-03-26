#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("client is disconnected")]
    Disconnected,

    #[error("canceled")]
    Canceled,

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
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Disconnected
    }
}

impl<T> From<tokio_util::sync::PollSendError<T>> for Error {
    fn from(_err: tokio_util::sync::PollSendError<T>) -> Self {
        Self::Disconnected
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(_err: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Disconnected
    }
}

impl std::str::FromStr for Error {
    type Err = std::convert::Infallible;

    fn from_str(error: &str) -> Result<Self, Self::Err> {
        Ok(Self::Other(error.into()))
    }
}
