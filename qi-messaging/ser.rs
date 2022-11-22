use super::*;

impl serde::Serialize for MagicCookie {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Self::VALUE.serialize(serializer)
    }
}
