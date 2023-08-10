use super::{Header, Message, ReadHeaderError, WriteHeaderError};
use crate::format;
use bytes::{Buf, BytesMut};
use tracing::instrument;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub(crate) struct Encoder;

impl tokio_util::codec::Encoder<Message> for Encoder {
    type Error = EncodeError;

    #[instrument(level = "trace", name = "encode", skip_all, err)]
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

    #[instrument(level = "trace", name = "decode", skip_all, err)]
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let msg = loop {
            match self.state {
                DecoderState::Header => match decode_header(src)? {
                    None => break None,
                    Some(header) => self.state = DecoderState::Body(header),
                },
                DecoderState::Body(header) => match decode_body(header.body_size, src) {
                    None => break None,
                    Some(body) => {
                        self.state = DecoderState::Header;
                        src.reserve(src.len());
                        break Some(Message::new(header, body));
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
    Body(Header),
}

#[instrument(level = "trace", skip_all)]
fn decode_header(src: &mut bytes::BytesMut) -> Result<Option<Header>, DecodeError> {
    if src.len() < Header::SIZE {
        src.reserve(Header::SIZE - src.len());
        return Ok(None);
    }

    let header = Header::read(&mut src.as_ref())?;
    src.advance(Header::SIZE);
    Ok(Some(header))
}

#[instrument(level = "trace", skip_all)]
fn decode_body(size: usize, src: &mut BytesMut) -> Option<format::Value> {
    if src.len() < size {
        src.reserve(size - src.len());
        return None;
    }
    let bytes = src.copy_to_bytes(size);
    let value = format::Value::from_bytes(bytes);
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message;
    use assert_matches::assert_matches;

    #[test]
    fn test_encoder_success() {
        let message = Message {
            id: message::Id(1),
            kind: message::Kind::Call,
            subject: message::Subject::default(),
            flags: message::Flags::all(),
            content: [1, 2, 3].into(),
        };
        let mut buf = BytesMut::new();
        let mut encoder = Encoder;
        let res = tokio_util::codec::Encoder::encode(&mut encoder, message.clone(), &mut buf);
        assert_matches!(res, Ok(()));

        let mut buf2 = vec![];
        message.write(&mut buf2).unwrap();
        assert_eq!(buf, buf2);
    }

    #[test]
    fn test_decoder_not_enough_data_for_header() {
        let data = [0x42, 0xde, 0xad];
        let mut buf = BytesMut::from_iter(data);
        let mut decoder = Decoder::new();
        let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
        assert_matches!(res, Ok(None));
    }

    #[test]
    fn test_decoder_not_enough_data_for_body() {
        let data = [
            0x42, 0xde, 0xad, 0x42, // cookie
            1, 0, 0, 0, // id
            5, 0, 0, 0, // size
            0, 0, 6, 2, // version, type, flags
            1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, // subject,
            1, 2, 3, // body
        ];
        let mut buf = BytesMut::from_iter(data);
        let mut decoder = Decoder::new();
        let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
        assert_matches!(res, Ok(None));
    }

    #[test]
    fn test_decoder_garbage_magic_cookie() {
        let data = [1; Header::SIZE];
        let mut buf = BytesMut::from_iter(data);
        let mut decoder = Decoder::new();
        let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
        assert_matches!(
            res,
            Err(DecodeError::ReadHeader(ReadHeaderError::MagicCookie(_)))
        );
    }

    #[test]
    fn test_decoder_success() {
        let data = [
            0x42, 0xde, 0xad, 0x42, // cookie
            1, 0, 0, 0, // id
            4, 0, 0, 0, // size
            0, 0, 6, 2, // version, type, flags
            1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, // subject,
            1, 2, 3, 4, // body
        ];
        let mut buf = BytesMut::from_iter(data);
        let mut decoder = Decoder::new();
        let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
        assert_matches!(res, Ok(Some(_msg)));
    }
}
