mod de;
pub mod message;
mod ser;

pub use de::{from_bytes, from_message, from_reader, Deserializer};
use futures::prelude::*;
pub use message::Message;
pub use ser::{to_bytes, to_message, to_writer, Serializer};
use std::str::Utf8Error;

pub fn message_stream_from_reader<'r, R>(reader: R) -> impl Stream<Item = Message> + 'r
where
    R: std::io::Read + 'r,
{
    stream::unfold(reader, |mut reader| async {
        let msg = from_reader(reader.by_ref()).ok()?;
        Some((msg, reader))
    })
}

pub fn message_sink_from_writer<'w, W>(writer: W) -> impl Sink<Message, Error = Error> + 'w
where
    W: std::io::Write + 'w,
{
    sink::unfold(writer, |mut writer, msg: Message| async move {
        to_writer(writer.by_ref(), &msg)?;
        Ok::<_, Error>(writer)
    })
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("size conversion failed: {0}")]
    BadSize(std::num::TryFromIntError),

    #[error("payload size was expected but none was found")]
    NoPayloadSize,

    #[error("list size must be known to be serialized")]
    UnknownListSize,

    #[error("unexpected message field {0}")]
    UnexpectedMessageField(&'static str),

    #[error("duplicate message field {0}")]
    DuplicateMessageField(&'static str),

    #[error("missing message field {0}")]
    MissingMessageField(&'static str),

    #[error("string data is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] Utf8Error),

    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::tests::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_to_from_bytes_invariant() {
        let sample = Serializable::sample();
        let bytes = to_bytes(&sample).unwrap();
        let sample2: Serializable = from_bytes(&bytes).unwrap();
        assert_eq!(sample, sample2);
    }

    #[test]
    fn dynamic_value_to_message() {
        use crate::typesystem::dynamic::Value;
        let input = vec![
            0x42, 0xde, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let message: Message = from_reader(input.as_slice()).unwrap();
        let dynamic: Value = from_message(&message).unwrap();
        assert_eq!(dynamic, Value::from("The robot is not localized"));
    }

    #[futures_test::test]
    async fn test_message_stream_from_reader() {
        let mut buf = Vec::new();
        let messages = message::tests::samples();
        for msg in &messages {
            to_writer(&mut buf, &msg).expect("message write error");
        }

        let stream = message_stream_from_reader(buf.as_slice());
        let stream_messages = stream.collect::<Vec<_>>().await;
        assert_eq!(stream_messages, messages);
    }

    #[futures_test::test]
    async fn test_message_sink_from_writer() {
        let mut buf = Vec::new();
        let messages = message::tests::samples();

        let mut sink = Box::pin(message_sink_from_writer(&mut buf));
        for msg in &messages {
            sink.send(msg.clone()).await.expect("sink send");
        }
        drop(sink);

        let mut reader = buf.as_slice();
        let mut actual_messages: Vec<Message> = Vec::new();
        for _i in 0..messages.len() {
            actual_messages.push(from_reader(&mut reader).expect("message read"));
        }
        assert_eq!(actual_messages, messages);
    }
}
