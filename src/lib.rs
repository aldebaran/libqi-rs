// TODO: #![warn(missing_docs)]

pub mod proto;

use futures::prelude::*;
use proto::message::{self, Message};

fn to_message_stream<'r, R>(reader: R) -> impl Stream<Item = Message> + 'r
where
    R: AsyncRead + Unpin + 'r,
{
    stream::unfold(reader, |mut reader| async {
        todo!()
        //let msg = Message::read(&mut reader).await.ok()?;
        //Some((msg, reader))
    })
}

fn to_message_sink<'w, W>(writer: W) -> impl Sink<Message, Error = proto::Error> + 'w
where
    W: AsyncWrite + Unpin + 'w,
{
    sink::unfold(writer, |mut writer, msg: Message| async move {
        todo!()
        //msg.write(&mut writer).await?;
        //Ok::<_, message::Error>(writer)
    })
}

pub mod server {
    use super::*;
    use std::pin::Pin;

    pub struct Remote<'a> {
        // OPTIMIZE: See if we could avoid using boxes here.
        stream: Pin<Box<dyn Stream<Item = Message> + Unpin + 'a>>,
        sink: Pin<Box<dyn Sink<Message, Error = proto::Error> + Unpin + 'a>>,
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

    #[test]
    async fn to_message_stream() {
        todo!()
        //let mut buf = Vec::new();
        //let messages = message::tests::samples();
        //for msg in &messages {
        //    msg.write(&mut buf).await.expect("message write error");
        //}

        //let stream = super::to_message_stream(buf.as_slice());
        //let stream_messages = stream.collect::<Vec<_>>().await;
        //assert_eq!(stream_messages, messages);
    }

    #[test]
    async fn to_message_sink() {
        todo!()
        //let mut buf = Vec::new();
        //let messages = message::tests::samples();

        //let mut sink = Box::pin(super::to_message_sink(&mut buf));
        //for msg in &messages {
        //    sink.send(msg.clone()).await.expect("sink send");
        //}
        //drop(sink);

        //let mut reader = buf.as_slice();
        //let mut actual_messages = Vec::new();
        //for _i in 0..messages.len() {
        //    actual_messages.push(Message::read(&mut reader).await.expect("message read"))
        //}
        //assert_eq!(actual_messages, messages);
    }

    //#[test]
    //async fn client_establish() {
    //    let mut reader = Vec::new();
    //    let mut writer = Vec::new();
    //    let client = Client::establish(reader.as_slice(), writer).await;
    //}
}
