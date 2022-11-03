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
    use typesystem::{
        value::{
            dynamic::{self, AnyValue},
            Value,
        },
        Type,
    };

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

    impl Value for Serializable {
        fn get_type<'t>() -> &'t Type {
            use std::sync::Once;
            use typesystem::r#type::tuple::Field;
            static mut TYPE: Option<Type> = None;
            static INIT: Once = Once::new();
            INIT.call_once(|| {
                let s0 = {
                    let t = Type::tuple_from_iter([
                        Type::Int8,
                        Type::UInt8,
                        Type::Int16,
                        Type::UInt16,
                        Type::Int32,
                        Type::UInt32,
                        Type::Int64,
                        Type::UInt64,
                        Type::Float,
                        Type::Double,
                    ]);
                    let r = Type::Raw;
                    let o = Type::option(Type::Bool);
                    let s = Type::named_tuple_from_iter("S1", [Type::String, Type::String]);
                    let l = Type::list(Type::String);
                    let m = Type::map(Type::Int32, Type::String);
                    Type::named_tuple_from_iter(
                        "S0",
                        [
                            Field::new("t", t),
                            Field::new("r", r),
                            Field::new("o", o),
                            Field::new("s", s),
                            Field::new("l", l),
                            Field::new("m", m),
                        ],
                    )
                };
                unsafe {
                    TYPE = Some(Type::named_tuple_from_iter("S", [s0]));
                }
            });
            unsafe { TYPE.as_ref().unwrap_unchecked() }
        }
    }

    pub fn sample_serializable_and_dynamic_value() -> (Serializable, AnyValue) {
        let s = Serializable::sample();
        let t = AnyValue::Tuple(dynamic::Tuple {
            name: None,
            elements: dynamic::tuple::Elements::Raw(vec![
                AnyValue::Int8(-8),
                AnyValue::UInt8(8),
                AnyValue::Int16(-16),
                AnyValue::UInt16(16),
                AnyValue::Int32(-32),
                AnyValue::UInt32(32),
                AnyValue::Int64(-64),
                AnyValue::UInt64(64),
                AnyValue::Float(32.32),
                AnyValue::Double(64.64),
            ]),
        });
        let r = AnyValue::Raw(vec![51, 52, 53, 54]);
        let o = AnyValue::Option {
            value_type: Type::option(Type::Bool),
            option: Some(Box::new(AnyValue::Bool(false))),
        };
        let s1 = AnyValue::Tuple(dynamic::Tuple {
            name: Some("S1".to_string()),
            elements: dynamic::tuple::Elements::Raw(vec![
                AnyValue::String("bananas".to_string()),
                AnyValue::String("oranges".to_string()),
            ]),
        });
        let l = AnyValue::List {
            value_type: Type::String,
            list: vec![
                AnyValue::String("cookies".to_string()),
                AnyValue::String("muffins".to_string()),
            ],
        };
        let m = AnyValue::Map {
            key_type: Type::Int32,
            value_type: Type::String,
            map: vec![
                (AnyValue::Int32(1), AnyValue::String("hello".to_string())),
                (AnyValue::Int32(2), AnyValue::String("world".to_string())),
            ],
        };
        let s0: AnyValue = dynamic::Tuple {
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
        let v = AnyValue::Tuple(dynamic::Tuple {
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
