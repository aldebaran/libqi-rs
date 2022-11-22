use bitflags::bitflags;

bitflags! {
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct Flags: u8 {
        const DYNAMIC_PAYLOAD = 0b00000001;
        const RETURN_TYPE = 0b00000010;
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
