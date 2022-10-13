use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct Flags: u8 {
        const DYNAMIC_PAYLOAD = 0b00000001;
        const RETURN_TYPE = 0b00000010;
    }
}

impl serde::ser::Serialize for Flags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bits.serialize(serializer)
    }
}

impl<'de> serde::de::Deserialize<'de> for Flags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self { bits })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    pub fn test_ser_de() {
        assert_tokens(&Flags::empty(), &[Token::U8(0)]);
        assert_tokens(&Flags::DYNAMIC_PAYLOAD, &[Token::U8(1)]);
        assert_tokens(&Flags::RETURN_TYPE, &[Token::U8(2)]);
        assert_tokens(
            &(Flags::RETURN_TYPE | Flags::DYNAMIC_PAYLOAD),
            &[Token::U8(3)],
        );
    }
}
