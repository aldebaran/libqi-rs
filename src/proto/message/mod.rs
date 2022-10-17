//! # Message
//!
//! ## Structure
//! ```text
//! ╔═══════════════╤═══════════════╤═══════════════╤═══════════════╗
//! ║       1       │       2       │       3       │       4       ║
//! ╟─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─╢
//! ║0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7║
//! ╠═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╧═╣
//! ║                         magic cookie                          ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                          identifier                           ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                         payload size                          ║
//! ╟───────────────────────────────┬───────────────┬───────────────╢
//! ║            version            │     type      │    flags      ║
//! ╟───────────────────────────────┴───────────────┴───────────────╢
//! ║                            service                            ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                            object                             ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                            action                             ║
//! ╟───────────────────────────────────────────────────────────────╢
//! ║                            payload                            ║
//! ║                             [...]                             ║
//! ╚═══════════════════════════════════════════════════════════════╝
//! ```
//!
//! ## Header fields
//!  - magic cookie: uint32, big endian, value = 0x42dead42
//!  - id: uint32, little endian
//!  - size/len: uint32, little endian, size of the payload. may be 0
//!  - version: uint16, little endian
//!  - type: uint8, little endian
//!  - flags: uint8, little endian
//!  - service: uint32, little endian
//!  - object: uint32, little endian
//!  - action: uint32, little endian
pub mod kind;
pub use kind::Kind;

pub mod flags;
pub use flags::Flags;

pub mod action;
pub use action::Action;

pub mod subject;
pub use subject::Subject;

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    #[error("bad message magic cookie")]
    BadMagicCookie,
    #[error("unsupported protocol version")]
    UnsupportedVersion,
    #[error("payload size too large")]
    PayloadSizeTooLarge,
}

#[derive(
    Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, serde::Serialize, serde::Deserialize,
)]
#[serde(rename = "qi.Message")]
pub struct Message {
    pub id: u32,
    pub version: u16,
    #[serde(rename = "type")]
    pub kind: Kind,
    pub flags: Flags,
    pub subject: Subject,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl Message {
    pub const CURRENT_VERSION: u16 = 0;

    pub fn new() -> Self {
        Self {
            id: 0,
            version: Self::CURRENT_VERSION,
            kind: Kind::None,
            flags: Flags::empty(),
            subject: Subject::default(),
            payload: Vec::new(),
        }
    }
}

impl Default for Message {
    fn default() -> Self {
        Message::new()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct MagicCookie;

impl MagicCookie {
    const VALUE: u32 = 0x42adde42;
}

impl serde::de::Expected for MagicCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl std::fmt::Display for MagicCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", Self::VALUE)
    }
}

impl serde::Serialize for MagicCookie {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Self::VALUE.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for MagicCookie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value != Self::VALUE {
            use serde::de;
            return Err(<D::Error as de::Error>::invalid_value(
                de::Unexpected::Unsigned(value.into()),
                &MagicCookie,
            ));
        }
        Ok(MagicCookie)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_bytes::Bytes;
    use serde_test::{assert_tokens, Token};

    pub fn samples() -> [Message; 3] {
        [
            Message {
                id: 123,
                version: Message::CURRENT_VERSION,
                kind: Kind::Post,
                flags: Flags::RETURN_TYPE,
                subject: subject::Subject::try_from_values(
                    subject::Service::Other(543.into()),
                    subject::Object::Other(32.into()),
                    action::BoundObject::Terminate.into(),
                )
                .unwrap(),
                payload: vec![1, 2, 3],
            },
            Message {
                id: 9034,
                version: Message::CURRENT_VERSION,
                kind: Kind::Event,
                flags: Flags::empty(),
                subject: subject::Subject::try_from_values(
                    subject::Service::Other(90934.into()),
                    subject::Object::Other(178.into()),
                    action::BoundObject::Metaobject.into(),
                )
                .unwrap(),
                payload: vec![],
            },
            Message {
                id: 21932,
                version: Message::CURRENT_VERSION,
                kind: Kind::Capability,
                flags: Flags::DYNAMIC_PAYLOAD,
                subject: subject::ServiceDirectory {
                    action: action::ServiceDirectory::UnregisterService,
                }
                .into(),
                payload: vec![100, 200, 255],
            },
        ]
    }

    #[test]
    fn test_message_ser_de() {
        let [msg, _, _] = samples();
        assert_tokens(
            &msg,
            &[
                Token::Struct {
                    name: "qi.Message",
                    len: 6,
                },
                Token::Str("id"),
                Token::U32(123),
                Token::Str("version"),
                Token::U16(0),
                Token::Str("type"),
                Token::U8(4),
                Token::Str("flags"),
                Token::U8(2),
                Token::Str("subject"),
                Token::Struct {
                    name: "Subject",
                    len: 3,
                },
                Token::Str("service"),
                Token::U32(543),
                Token::Str("object"),
                Token::U32(32),
                Token::Str("action"),
                Token::U32(3),
                Token::StructEnd, // subject
                Token::Str("payload"),
                Token::Bytes(&[1, 2, 3]),
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn test_message_de_bad_cookie() {
        use serde::de::{Deserialize, IntoDeserializer};
        #[derive(thiserror::Error, Debug, PartialEq, Eq)]
        #[error("{0}")]
        struct Error(String);

        impl serde::de::Error for Error {
            fn custom<T>(msg: T) -> Self
            where
                T: std::fmt::Display,
            {
                Self(msg.to_string())
            }
        }

        let de = 0x42addf42u32.into_deserializer();
        assert_eq!(
            MagicCookie::deserialize(de),
            Err(Error(
                "invalid value: integer `1118691138`, expected 0x42adde42".into()
            ))
        );
    }
}
