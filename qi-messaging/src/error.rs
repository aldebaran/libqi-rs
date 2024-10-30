// TODO: Make an Error trait that allows checking if an error type is "canceled" so that we may send
// back a canceled message to a client instead of an error.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("messaging link has been lost")]
    LinkLost(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("{0}")]
    CallError(String),

    #[error("the call request has been canceled")]
    CallCanceled,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::LinkLost(err.into())
    }
}
