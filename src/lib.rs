// TODO: #![warn(missing_docs)]

pub mod proto;
pub mod typesystem;

//use futures::prelude::*;

//pub mod server {
//    use super::*;
//    use std::pin::Pin;
//
//    pub struct Remote<'a> {
//        // OPTIMIZE: See if we could avoid using boxes here.
//        stream: Pin<Box<dyn Stream<Item = proto::Message> + Unpin + 'a>>,
//        sink: Pin<Box<dyn Sink<proto::Message, Error = proto::Error> + Unpin + 'a>>,
//    }
//
//    impl<'a> Remote<'a> {
//        pub fn from_read_write<R, W>(reader: R, writer: W) -> Self
//        where
//            R: std::io::Read + 'a,
//            W: std::io::Write + 'a,
//        {
//            let _stream = Box::pin(proto::message_stream_from_reader(reader));
//            let _sink = Box::pin(proto::message_sink_from_writer(writer));
//            todo!()
//        }
//    }
//}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use typesystem::dynamic::{self, Value};

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct S0 {
        t: (i8, u8, i16, u16, i32, u32, i64, u64, f32, f64),
        #[serde(with = "serde_bytes")]
        r: Vec<u8>,
        o: Option<bool>,
        s: S1,
        l: Vec<String>,
        m: BTreeMap<i32, String>,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct S1(String, String);

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    pub struct Serializable(S0);

    impl Serializable {
        pub fn sample() -> Self {
            Self(S0 {
                t: (-8, 8, -16, 16, -32, 32, -64, 64, 32.32, 64.64),
                r: vec![51, 52, 53, 54],
                o: Some(false),
                s: S1("bananas".to_string(), "oranges".to_string()),
                l: vec!["cookies".to_string(), "muffins".to_string()],
                m: {
                    let mut m = BTreeMap::new();
                    m.insert(1, "hello".to_string());
                    m.insert(2, "world".to_string());
                    m
                },
            })
        }
    }

    pub fn sample_serializable_and_dynamic_value() -> (Serializable, Value) {
        let s = Serializable::sample();
        let t = Value::Tuple(dynamic::Tuple {
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

    //#[test]
    //async fn client_establish() {
    //    let mut reader = Vec::new();
    //    let mut writer = Vec::new();
    //    let client = Client::establish(reader.as_slice(), writer).await;
    //}
}
