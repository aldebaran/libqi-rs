use super::Value;
use crate::dynamic;

impl serde::Serialize for Value<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Unit => ().serialize(serializer),
            Value::Bool(v) => v.serialize(serializer),
            Value::Int8(v) => v.serialize(serializer),
            Value::UInt8(v) => v.serialize(serializer),
            Value::Int16(v) => v.serialize(serializer),
            Value::UInt16(v) => v.serialize(serializer),
            Value::Int32(v) => v.serialize(serializer),
            Value::UInt32(v) => v.serialize(serializer),
            Value::Int64(v) => v.serialize(serializer),
            Value::UInt64(v) => v.serialize(serializer),
            Value::Float32(v) => v.serialize(serializer),
            Value::Float64(v) => v.serialize(serializer),
            Value::String(v) => v.serialize(serializer),
            Value::Raw(v) => serializer.serialize_bytes(v),
            Value::Option(opt) => opt.serialize(serializer),
            Value::List(list) => list.serialize(serializer),
            Value::Map(map) => map.serialize(serializer),
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
