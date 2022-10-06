pub mod proto;

use futures::prelude::*;
use proto::message::{self, Message};

fn to_message_stream<'r, R>(reader: R) -> impl Stream<Item = Message> + 'r
where
    R: AsyncRead + Unpin + 'r,
{
    stream::unfold(reader, |mut reader| async {
        let msg = Message::read(&mut reader).await.ok()?;
        Some((msg, reader))
    })
}

fn to_message_sink<'w, W>(writer: W) -> impl Sink<Message, Error = message::Error> + 'w
where
    W: AsyncWrite + Unpin + 'w,
{
    sink::unfold(writer, |mut writer, msg: Message| async move {
        msg.write(&mut writer).await?;
        Ok::<_, message::Error>(writer)
    })
}

pub mod server {
    use super::*;
    use std::pin::Pin;

    pub struct Remote<'a> {
        // OPTIMIZE: See if we could avoid using boxes here.
        stream: Pin<Box<dyn Stream<Item = Message> + Unpin + 'a>>,
        sink: Pin<Box<dyn Sink<Message, Error = message::Error> + Unpin + 'a>>,
    }

    impl<'a> Remote<'a> {
        pub fn from_read_write<R, W>(reader: R, writer: W) -> Self
        where
            R: AsyncRead + Unpin + 'a,
            W: AsyncWrite + Unpin + 'a,
        {
            let _stream = Box::pin(to_message_stream(reader));
            let _sink = Box::pin(to_message_sink(writer));
            todo!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_test::test;
    use pretty_assertions::assert_eq;

    fn examples_message() -> [Message; 3] {
        [
            Message {
                id: 123,
                kind: message::Kind::Post,
                flags: message::Flags::RETURN_TYPE,
                target: message::Target::BoundObject {
                    service: 543,
                    object: 32,
                    action: message::BoundObjectAction::Terminate,
                },
                payload: vec![1, 2, 3],
            },
            Message {
                id: 9034,
                kind: message::Kind::Event,
                flags: message::Flags::empty(),
                target: message::Target::BoundObject {
                    service: 90934,
                    object: 178,
                    action: message::BoundObjectAction::Metaobject,
                },
                payload: vec![],
            },
            Message {
                id: 21932,
                kind: message::Kind::Capability,
                flags: message::Flags::DYNAMIC_PAYLOAD,
                target: message::Target::ServiceDirectory(
                    message::ServiceDirectoryAction::UnregisterService,
                ),
                payload: vec![100, 200, 255],
            },
        ]
    }

    #[test]
    async fn to_message_stream() {
        let mut buf = Vec::new();
        let messages = examples_message();
        for msg in &messages {
            msg.write(&mut buf).await.expect("message write error");
        }

        let stream = super::to_message_stream(buf.as_slice());
        let stream_messages = stream.collect::<Vec<_>>().await;
        assert_eq!(stream_messages, messages);
    }

    #[test]
    async fn to_message_sink() {
        let mut buf = Vec::new();
        let messages = examples_message();

        let mut sink = Box::pin(super::to_message_sink(&mut buf));
        for msg in &messages {
            sink.send(msg.clone()).await.expect("sink send");
        }
        drop(sink);

        let mut reader = buf.as_slice();
        let mut actual_messages = Vec::new();
        for _i in 0..messages.len() {
            actual_messages.push(Message::read(&mut reader).await.expect("message read"))
        }
        assert_eq!(actual_messages, messages);
    }

    //#[test]
    //async fn client_establish() {
    //    let mut reader = Vec::new();
    //    let mut writer = Vec::new();
    //    let client = Client::establish(reader.as_slice(), writer).await;
    //}
}
