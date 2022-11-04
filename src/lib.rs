// TODO: #![warn(missing_docs)]

pub mod proto;
pub mod typesystem;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use indexmap::indexmap;
    use std::collections::BTreeMap;
    use typesystem::{
        value::{dynamic::AnyValue, Value},
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
            static mut TYPE: Option<Type> = None;
            static INIT: Once = Once::new();
            INIT.call_once(|| {
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
                unsafe {
                    TYPE = Some(Type::TupleStruct {
                        name: "S".into(),
                        elements: vec![s0],
                    });
                }
            });
            unsafe { TYPE.as_ref().unwrap_unchecked() }
        }
    }

    pub fn sample_serializable_and_anyvalue() -> (Serializable, AnyValue) {
        let s = Serializable::sample();
        let t = AnyValue::Tuple(vec![
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
        ]);
        let r = AnyValue::Raw(vec![51, 52, 53, 54]);
        let o = AnyValue::Option {
            value_type: Type::Bool,
            option: Some(AnyValue::Bool(false).into()),
        };
        let s1 = AnyValue::TupleStruct {
            name: "S1".into(),
            elements: vec![
                AnyValue::String("bananas".into()),
                AnyValue::String("oranges".into()),
            ],
        };
        let l = AnyValue::List {
            value_type: Type::String,
            list: vec![
                AnyValue::String("cookies".into()),
                AnyValue::String("muffins".into()),
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
        let s0 = AnyValue::Struct {
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
        let v = AnyValue::TupleStruct {
            name: "Serializable".into(),
            elements: vec![s0],
        };
        (s, v)
    }
}
