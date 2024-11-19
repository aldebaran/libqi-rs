#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("messaging link has been lost")]
    LinkLost(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("{0}")]
    CallError(String),

    #[error("the call request has been canceled")]
    CallCanceled,
}

impl Error {
    pub fn link_lost<E>(err: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::LinkLost(err.into())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::LinkLost(err.into())
    }
}
