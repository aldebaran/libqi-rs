use super::{Header, Message, ReadHeaderError, WriteHeaderError};
use bytes::{Buf, Bytes, BytesMut};
use tracing::instrument;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub(crate) struct Encoder;

impl tokio_util::codec::Encoder<Message> for Encoder {
    type Error = EncodeError;

    #[instrument(name = "encode", skip_all, err)]
    fn encode(&mut self, msg: Message, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        dst.reserve(msg.size());
        msg.write(dst)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EncodeError {
    #[error("write header error")]
    WriteHeader(#[from] WriteHeaderError),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub(crate) struct Decoder {
    state: DecoderState,
}

impl Decoder {
    pub(crate) fn new() -> Self {
        Self {
            state: DecoderState::Header,
        }
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl tokio_util::codec::Decoder for Decoder {
    type Item = Message;
    type Error = DecodeError;

    #[instrument(name = "decode", skip_all, err, level = "debug")]
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let msg = loop {
            match self.state {
                DecoderState::Header => match decode_header(src)? {
                    None => break None,
                    Some(header) => self.state = DecoderState::Payload(header),
                },
                DecoderState::Payload(header) => match decode_payload(header.payload_size, src) {
                    None => break None,
                    Some(payload) => {
                        self.state = DecoderState::Header;
                        src.reserve(src.len());
                        break Some(Message::new(header, payload));
                    }
                },
            }
        };
        Ok(msg)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DecodeError {
    #[error("read header error")]
    ReadHeader(#[from] ReadHeaderError),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
enum DecoderState {
    Header,
    Payload(Header),
}

#[instrument(skip_all, level = "debug")]
fn decode_header(src: &mut bytes::BytesMut) -> Result<Option<Header>, DecodeError> {
    if src.len() < Header::SIZE {
        src.reserve(Header::SIZE - src.len());
        return Ok(None);
    }

    let header = Header::read(&mut src.as_ref())?;
    src.advance(Header::SIZE);
    Ok(Some(header))
}

#[instrument(skip_all, level = "debug")]
fn decode_payload(size: usize, src: &mut BytesMut) -> Option<Bytes> {
    if src.len() < size {
        src.reserve(size - src.len());
        return None;
    }
    Some(src.copy_to_bytes(size))
}

#[cfg(test)]
mod tests {
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
