#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectionError {
    #[error("message decoding error")]
    Decode(#[from] qi_messaging::codec::DecodeError),

    #[error("message encoding error")]
    Encode(#[from] qi_messaging::codec::EncodeError),

    #[error("IO error")]
    Io(#[from] std::io::Error),
}
