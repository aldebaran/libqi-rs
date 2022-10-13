use bitflags::bitflags;

bitflags! {
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    pub struct Flags: u8 {
        const DYNAMIC_PAYLOAD = 0b00000001;
        const RETURN_TYPE = 0b00000010;
    }
}

//impl Flags {
//    async fn write<W>(&self, mut writer: W) -> Result<()>
//    where
//        W: AsyncWrite + Unpin,
//    {
//        let bytes = &self.bits().to_le_bytes();
//        writer.write_all(bytes).await?;
//        Ok(())
//    }
//
//    async fn read<R>(reader: R) -> Result<Self>
//    where
//        R: AsyncRead + Unpin,
//    {
//        let val = read_u8(reader).await?;
//        Flags::from_bits(val).ok_or(Error::InvalidValue)
//    }
//}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    pub fn test_ser_de() {
        assert_tokens(
            &Flags::empty(),
            &[
                Token::Struct {
                    name: "Flags",
                    len: 1,
                },
                Token::Str("bits"),
                Token::U8(0),
                Token::StructEnd,
            ],
        );
        assert_tokens(
            &Flags::DYNAMIC_PAYLOAD,
            &[
                Token::Struct {
                    name: "Flags",
                    len: 1,
                },
                Token::Str("bits"),
                Token::U8(1),
                Token::StructEnd,
            ],
        );
        assert_tokens(
            &Flags::RETURN_TYPE,
            &[
                Token::Struct {
                    name: "Flags",
                    len: 1,
                },
                Token::Str("bits"),
                Token::U8(2),
                Token::StructEnd,
            ],
        );
        assert_tokens(
            &(Flags::RETURN_TYPE | Flags::DYNAMIC_PAYLOAD),
            &[
                Token::Struct {
                    name: "Flags",
                    len: 1,
                },
                Token::Str("bits"),
                Token::U8(3),
                Token::StructEnd,
            ],
        );
    }
}
