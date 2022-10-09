//use futures::{AsyncRead, AsyncWrite, AsyncWriteExt};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(
    FromPrimitive,
    ToPrimitive,
    Debug,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
)]
#[repr(u8)]
pub enum Kind {
    None = 0,
    Call = 1,
    Reply = 2,
    Error = 3,
    Post = 4,
    Event = 5,
    Capability = 6,
    Cancel = 7,
    Canceled = 8,
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    InvalidValue(u8),
}

impl Default for Kind {
    fn default() -> Self {
        Self::None
    }
}

impl std::convert::Into<u8> for Kind {
    fn into(self) -> u8 {
        self.to_u8().unwrap()
    }
}

impl std::convert::TryFrom<u8> for Kind {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Error> {
        Self::from_u8(value).ok_or(Error::InvalidValue(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use pretty_assertions::assert_eq;
    use serde_test::{assert_tokens, Token};

    #[test]
    pub fn test_into_u8() {
        let u: u8 = Kind::Cancel.into();
        assert_eq!(u, 7u8);
    }

    #[test]
    pub fn test_try_from_u8() {
        let k = Kind::try_from(5).unwrap();
        assert_eq!(k, Kind::Event);
        let k = Kind::try_from(42);
        assert_matches!(k, Err(Error::InvalidValue(42)));
    }

    #[test]
    pub fn test_ser_de() {
        assert_tokens(
            &Kind::Post,
            &[
                Token::Enum { name: "Kind" },
                Token::Str("Post"),
                Token::Unit,
            ],
        );
        assert_tokens(
            &Kind::Capability,
            &[
                Token::Enum { name: "Kind" },
                Token::Str("Capability"),
                Token::Unit,
            ],
        );
    }
}
