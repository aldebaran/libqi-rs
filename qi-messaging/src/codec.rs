use crate::message::{self, Message};
use std::io::Cursor;

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub struct MessageCodec;

impl tokio_util::codec::Decoder for MessageCodec {
    type Item = Message;
    type Error = DecodeError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        use bytes::Buf;
        let mut cursor = Cursor::new(src.as_ref());

        use message::{HeaderReadError, NotEnoughDataError, PayloadReadError, ReadError};
        match Message::read(&mut cursor) {
            Err(err) => match err {
                ReadError::Header(header_err) => match header_err {
                    HeaderReadError::NotEnoughData(NotEnoughDataError { expected, actual }) => {
                        src.reserve(expected - actual);
                        Ok(None)
                    }
                    header_err => Err(DecodeError::from(header_err)),
                },
                ReadError::Payload(PayloadReadError(NotEnoughDataError { expected, actual })) => {
                    src.reserve(expected - actual);
                    Ok(None)
                }
            },
            Ok(msg) => {
                let pos = cursor.position() as usize;
                src.advance(pos);
                Ok(Some(msg))
            }
        }
    }
}

impl tokio_util::codec::Encoder<Message> for MessageCodec {
    type Error = EncodeError;

    fn encode(&mut self, msg: Message, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        dst.reserve(msg.size());
        msg.write(dst)?;
        Ok(())
    }
}

pub use message::HeaderReadError as MessageHeaderReadError;

#[derive(thiserror::Error, Debug)]
pub enum DecodeError {
    #[error("error reading message header: {0}")]
    MessageHeader(#[from] MessageHeaderReadError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub use message::HeaderWriteError as MessageHeaderWriteError;

#[derive(thiserror::Error, Debug)]
pub enum EncodeError {
    #[error("error writing message header: {0}")]
    MessageHeader(#[from] MessageHeaderWriteError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
