use crate::message::{self, EndOfInputError, HeaderReadError, HeaderWriteError, Message};
use bytes::Buf;
use std::error::Error;
use tracing::{instrument, trace, warn};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub(crate) enum Decoder {
    Header,
    Payload(message::Header),
}

impl Decoder {
    pub(crate) fn new() -> Self {
        Self::Header
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

#[instrument(skip_all)]
fn decode_header(src: &mut bytes::BytesMut) -> Option<message::Header> {
    if src.len() < message::Header::SIZE {
        src.reserve(message::Header::SIZE - src.len());
        return None;
    }

    match message::Header::read(&mut src.as_ref()) {
        Err(HeaderReadError::EndOfInput(err)) => unreachable!(
            "logic error: the buffer of bytes should be large enough for a header, err={err}"
        ),
        Err(error @ HeaderReadError::InvalidMessageCookieValue(_)) => {
            trace!(
                error = &error as &dyn Error,
                "message header decoding error, skipping magic cookie bytes"
            );
            src.advance(message::MagicCookie::SIZE);
            None
        }
        Err(error) => {
            trace!(
                error = &error as &dyn Error,
                "message header decoding error, skipping header bytes"
            );
            src.advance(message::Header::SIZE);
            None
        }
        Ok(header) => {
            src.advance(message::Header::SIZE);
            Some(header)
        }
    }
}

#[instrument(skip_all)]
fn decode_payload(size: usize, src: &mut bytes::BytesMut) -> Option<message::Payload> {
    if src.len() < size {
        src.reserve(size - src.len());
        return None;
    }

    match message::Payload::read(size, &mut src.as_ref()) {
        Err(message::PayloadReadError(EndOfInputError { .. })) => unreachable!("logic error: the buffer of bytes should be large enough for the payload of size {size}"),
        Ok(payload) => {
            src.advance(payload.size());
            Some(payload)
        },
    }
}

impl tokio_util::codec::Decoder for Decoder {
    type Item = Message;
    type Error = std::io::Error;

    #[instrument(name = "decode", skip_all)]
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let msg = loop {
            match self {
                Self::Header => {
                    trace!("decoding header");
                    match decode_header(src) {
                        None => break None,
                        Some(header) => *self = Self::Payload(header),
                    }
                }
                Self::Payload(header) => {
                    trace!(?header, "decoding payload");
                    match decode_payload(header.payload_size(), src) {
                        None => break None,
                        Some(payload) => {
                            let header = std::mem::take(header);
                            *self = Self::Header;
                            src.reserve(src.len());
                            break Some(Message::new(header, payload));
                        }
                    }
                }
            }
        };
        Ok(msg)
    }
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub(crate) struct Encoder;

impl tokio_util::codec::Encoder<Message> for Encoder {
    type Error = std::io::Error;

    #[instrument(name = "encode", skip_all)]
    fn encode(&mut self, msg: Message, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        dst.reserve(msg.size());
        if let Err(error @ HeaderWriteError::PayloadSizeCannotBeRepresentedAsU32(_)) =
            msg.write(dst)
        {
            warn!(
                error = &error as &dyn Error,
                "message encoding error, discarding message"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_not_enough_data_for_header() {
        todo!()
    }

    #[test]
    fn test_decoder_not_enough_data_for_payload() {
        todo!()
    }

    #[test]
    fn test_decoder_garbage() {
        todo!()
    }

    #[test]
    fn test_decoder_success() {
        todo!()
    }

    #[test]
    fn test_encoder_bad_payload_size() {
        todo!()
    }

    #[test]
    fn test_encoder_success() {
        todo!()
    }
}
