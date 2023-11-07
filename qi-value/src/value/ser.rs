use super::Value;
use crate::dynamic;

impl serde::Serialize for Value<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;
        match self {
            Value::Unit => serializer.serialize_unit(),
            Value::Bool(v) => serializer.serialize_bool(*v),
            Value::Int8(v) => serializer.serialize_i8(*v),
            Value::UInt8(v) => serializer.serialize_u8(*v),
            Value::Int16(v) => serializer.serialize_i16(*v),
            Value::UInt16(v) => serializer.serialize_u16(*v),
            Value::Int32(v) => serializer.serialize_i32(*v),
            Value::UInt32(v) => serializer.serialize_u32(*v),
            Value::Int64(v) => serializer.serialize_i64(*v),
            Value::UInt64(v) => serializer.serialize_u64(*v),
            Value::Float32(v) => serializer.serialize_f32(v.0),
            Value::Float64(v) => serializer.serialize_f64(v.0),
            Value::String(v) => {
                serializer.serialize_str(std::str::from_utf8(v).map_err(S::Error::custom)?)
            }
            Value::Raw(v) => serializer.serialize_bytes(v),
            Value::Option(opt) => match opt {
                Some(v) => serializer.serialize_some(v),
                None => serializer.serialize_none(),
            },
            Value::List(list) => serializer.collect_seq(list),
            Value::Map(map) => serializer.collect_map(map.iter().map(|(k, v)| (k, v))),
            Value::Tuple(elements) => {
                use serde::ser::SerializeTuple;
                let mut serializer = serializer.serialize_tuple(elements.len())?;
                for element in elements {
                    serializer.serialize_element(&element)?;
                }
                serializer.end()
            }
            Value::Object(obj) => obj.serialize(serializer),
            Value::Dynamic(val) => dynamic::serialize(val, serializer),
        }
    }
}
