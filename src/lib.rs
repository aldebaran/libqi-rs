// TODO: #![warn(missing_docs)]

pub mod proto;
pub mod typesystem;
pub use typesystem::{r#type, value};

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
    use pretty_assertions::assert_eq;
    use typesystem::value::{dynamic, Value};

    pub fn sample_serializable_and_dynamic_value() -> (proto::tests::Serializable, Value) {
        let s = proto::tests::Serializable::sample();
        let t = value::Value::Tuple(dynamic::Tuple {
            name: None,
            elements: dynamic::tuple::Elements::Raw(vec![
                Value::Int8(-8),
                Value::UInt8(8),
                Value::Int16(-16),
                Value::UInt16(16),
                Value::Int32(-32),
                Value::UInt32(32),
                Value::Int64(-64),
                Value::UInt64(64),
                Value::Float(32.32),
                Value::Double(64.64),
            ]),
        });
        let r = Value::Raw(vec![51, 52, 53, 54]);
        let o = Value::Optional(Some(Box::new(Value::Bool(false))));
        let s1 = Value::Tuple(dynamic::Tuple {
            name: Some("S1".to_string()),
            elements: dynamic::tuple::Elements::Raw(vec![
                Value::String("bananas".to_string()),
                Value::String("oranges".to_string()),
            ]),
        });
        let l = Value::List(vec![
            Value::String("cookies".to_string()),
            Value::String("muffins".to_string()),
        ]);
        let m = Value::Map(vec![
            (Value::Int32(1), Value::String("hello".to_string())),
            (Value::Int32(2), Value::String("world".to_string())),
        ]);
        let s0: Value = dynamic::Tuple {
            name: Some("S0".to_string()),
            elements: [
                dynamic::tuple::Field {
                    name: "t".to_string(),
                    element: t,
                },
                dynamic::tuple::Field {
                    name: "r".to_string(),
                    element: r,
                },
                dynamic::tuple::Field {
                    name: "o".to_string(),
                    element: o,
                },
                dynamic::tuple::Field {
                    name: "s".to_string(),
                    element: s1,
                },
                dynamic::tuple::Field {
                    name: "l".to_string(),
                    element: l,
                },
                dynamic::tuple::Field {
                    name: "m".to_string(),
                    element: m,
                },
            ]
            .into_iter()
            .collect(),
        }
        .into();
        let v = Value::Tuple(dynamic::Tuple {
            name: Some("Serializable".to_string()),
            elements: [s0].into_iter().collect(),
        });
        (s, v)
    }

    #[test]
    fn dynamic_to_message() {
        let input = vec![
            0x42, 0xde, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let message: Message = proto::from_reader(input.as_slice()).unwrap();
        let dynamic: Value = proto::from_message(&message).unwrap();
        assert_eq!(dynamic, Value::from("The robot is not localized"));
    }

    #[futures_test::test]
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

    #[futures_test::test]
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
