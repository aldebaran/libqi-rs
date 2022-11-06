// TODO: #![warn(missing_docs)]

pub mod format;
pub mod message;
pub mod reflect;
pub mod signature;
pub mod r#type;
pub mod value;

pub use format::{
    from_bytes, from_message, from_reader, to_bytes, to_message, to_writer, Deserializer, Error,
    Result, Serializer,
};
pub use message::Message;
pub use r#type::Type;
pub use reflect::Reflect;
pub use signature::Signature;
pub use value::{from_value_ref, to_value, Value};

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use indexmap::indexmap;
    use std::collections::BTreeMap;

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

        pub fn sample_as_value() -> Value {
            let t = Value::Tuple(vec![
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
            ]);
            let r = Value::Raw(vec![51, 52, 53, 54]);
            let o = Value::Option(Some(Value::Bool(false).into()));
            let s1 = Value::TupleStruct {
                name: "S1".into(),
                elements: vec![
                    Value::String("bananas".into()),
                    Value::String("oranges".into()),
                ],
            };
            let l = Value::List(vec![
                Value::String("cookies".into()),
                Value::String("muffins".into()),
            ]);
            let m = Value::Map(vec![
                (Value::Int32(1), Value::String("hello".to_string())),
                (Value::Int32(2), Value::String("world".to_string())),
            ]);
            let s0 = Value::Struct {
                name: "S0".into(),
                fields: indexmap![
                    "t".into() => t,
                    "r".into() => r,
                    "o".into() => o,
                    "s".into() => s1,
                    "l".into() => l,
                    "m".into() => m,
                ],
            };
            Value::TupleStruct {
                name: "Serializable".into(),
                elements: vec![s0],
            }
        }
    }

    impl Reflect for Serializable {
        fn get_type<'t>() -> &'t Type {
            use once_cell::sync::OnceCell;
            static TYPE: OnceCell<Type> = OnceCell::new();
            TYPE.get_or_init(|| {
                let s0 = {
                    let t = Type::Tuple(vec![
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
                    let o = Type::Option(Type::Bool.into());
                    let s = Type::TupleStruct {
                        name: "S1".into(),
                        elements: vec![Type::String, Type::String],
                    };
                    let l = Type::List(Type::String.into());
                    let m = Type::Map {
                        key: Type::Int32.into(),
                        value: Type::String.into(),
                    };
                    Type::Struct {
                        name: "S0".into(),
                        fields: indexmap! {
                            "t".into() => t,
                            "r".into() => r,
                            "o".into() => o,
                            "s".into() => s,
                            "l".into() => l,
                            "m".into() => m,
                        },
                    }
                };
                Type::TupleStruct {
                    name: "S".into(),
                    elements: vec![s0],
                }
            })
        }
    }
}
