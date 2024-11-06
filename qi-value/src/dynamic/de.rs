use crate::{value::de::ValueType, Signature, Value};

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
