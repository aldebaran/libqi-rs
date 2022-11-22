use super::*;

impl<'de> serde::Deserialize<'de> for MagicCookie {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value != Self::VALUE {
            use serde::de;
            return Err(<D::Error as de::Error>::invalid_value(
                de::Unexpected::Unsigned(value.into()),
                &format!("the magic cookie {}", MagicCookie).as_str(),
            ));
        }
        Ok(MagicCookie)
    }
}
