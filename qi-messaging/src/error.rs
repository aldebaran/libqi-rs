use qi_format as format;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("client is disconnected")]
    Disconnected,

    #[error("canceled")]
    Canceled,

    #[error("{0}")]
    Message(String),

    #[error("format error")]
    Format(#[from] format::Error),

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

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(_err: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Disconnected
    }
}
