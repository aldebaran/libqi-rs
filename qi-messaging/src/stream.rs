use std::io::Cursor;

use crate::message::{self, Message};
use pin_project_lite::pin_project;
use tokio_util::codec::Framed;

pin_project! {
    #[derive(Debug)]
    pub struct Stream<IO> {
        #[pin]
        io: Framed<IO, MessageCodec>,
    }
}

impl<IO> Stream<IO>
where
    IO: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    pub fn new(io: IO) -> Self {
        Self {
            io: Framed::new(io, MessageCodec),
        }
    }
}

impl<IO> futures::Sink<Message> for Stream<IO>
where
    IO: tokio::io::AsyncWrite,
{
    type Error = EncodeError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.project().io.poll_ready(cx)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.project().io.start_send(item)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.project().io.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.project().io.poll_close(cx)
    }
}

impl<T> futures::Stream for Stream<T>
where
    T: tokio::io::AsyncRead,
{
    type Item = Result<Message, DecodeError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().io.poll_next(cx)
    }
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
struct MessageCodec;

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
