use super::{AsDynamic, AsDynamicOwned, Fields, SERDE_STRUCT_NAME};
use crate::{value::de::ValueType, Dynamic, FromValue, Signature, Value};

impl<'de, T> serde::Deserialize<'de> for Dynamic<T>
where
    T: FromValue<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize(deserializer).map(Self)
    }
}

pub(crate) struct DynamicVisitor;

impl<'de> serde::de::Visitor<'de> for DynamicVisitor {
    type Value = Value<'de>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a dynamic value")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        // Signature
        let signature: Signature = seq
            .next_element()?
            .ok_or_else(|| Error::invalid_length(0, &self))?;
        let value_type = signature.into_type();

        // Value
        let value = seq
            .next_element_seed(ValueType(value_type.as_ref()))?
            .ok_or_else(|| Error::invalid_length(1, &self))?;

        Ok(value)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Signature,
            Value,
        }
        use serde::de::Error;

        let signature: Signature = match map.next_key()? {
            Some(Field::Signature) => map.next_value(),
            _ => Err(Error::missing_field("signature")),
        }?;
        let value_type = signature.into_type();
        let value = match map.next_key()? {
            Some(Field::Value) => map.next_value_seed(ValueType(value_type.as_ref())),
            _ => Err(Error::missing_field("value")),
        }?;
        Ok(value)
    }
}

impl<'de, T> serde_with::DeserializeAs<'de, T> for AsDynamic
where
    T: FromValue<'de>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self::deserialize(deserializer)
    }
}

impl<'de> serde_with::DeserializeAs<'de, Value<'static>> for AsDynamicOwned {
    fn deserialize_as<D>(deserializer: D) -> Result<Value<'static>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_value(deserializer).map(Value::into_owned)
    }
}

fn deserialize_value<'de, D>(deserializer: D) -> Result<Value<'de>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_struct(SERDE_STRUCT_NAME, &Fields::KEYS, DynamicVisitor)
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromValue<'de>,
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    deserialize_value(deserializer)?
        .cast_into()
        .map_err(|err| D::Error::custom(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn u32_from_json_number() {
        let value: u32 = deserialize_value(json!({ "signature": "I", "value": 42 }))
            .unwrap()
            .cast_into()
            .expect("value is not a u32");
        assert_eq!(value, 42)
    }
}
