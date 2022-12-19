//! Module defining `qi` messages related types and functions.
//!
//! ## Message Structure
//! ```text
//! ╔═══════════════════════════════════════════════════════════════════╗
//! ║                              HEADER                               ║
//! ╠═╤═══════════════╤═══════════════╤═══════════════╤═══════════════╤═╣
//! ║ │       0       │       1       │       2       │       3       │ ║
//! ║ ├─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┼─┬─┬─┬─┬─┬─┬─┬─┤ ║
//! ║ │0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│0│1│2│3│4│5│6│7│ ║
//! ║ ├─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┤ ║
//! ║ │                        magic cookie                           │ ║
//! ║ ├───────────────────────────────────────────────────────────────┤ ║
//! ║ │                          identifier                           │ ║
//! ║ ├───────────────────────────────────────────────────────────────┤ ║
//! ║ │                         payload size                          │ ║
//! ║ ├───────────────────────────────┬───────────────┬───────────────┤ ║
//! ║ │            version            │     type      │    flags      │ ║
//! ║ ├───────────────────────────────┴───────────────┴───────────────┤ ║
//! ║ │                            service                            │ ║
//! ║ ├───────────────────────────────────────────────────────────────┤ ║
//! ║ │                            object                             │ ║
//! ║ ├───────────────────────────────────────────────────────────────┤ ║
//! ║ │                            action                             │ ║
//! ╠═╧═══════════════════════════════════════════════════════════════╧═╣
//! ║                             PAYLOAD                               ║
//! ╚═══════════════════════════════════════════════════════════════════╝
//! ```
//!
//! ### Message header fields
//!  - magic cookie: uint32
//!  - id: uint32
//!  - size/len: uint32, size of the payload. may be 0
//!  - version: uint16
//!  - type: uint8
//!  - flags: uint8
//!  - service: uint32
//!  - object: uint32
//!  - action: uint32

pub mod kind;
pub use kind::Kind;

pub mod flags;
pub use flags::Flags;

pub mod subject;
pub use subject::Subject;

pub mod de;
pub mod ser;

use derive_more::{From, Into};
use derive_new::new;
use std::borrow::Cow;

// TODO: Split header with payload.
#[derive(
    new, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, serde::Serialize, serde::Deserialize,
)]
pub struct Message<'m> {
    id: Id,
    version: Version,
    kind: Kind,
    flags: Flags,
    subject: Subject,
    #[serde(with = "serde_bytes")]
    payload: Payload<'m>,
}

#[derive(
    new,
    Default,
    Clone,
    Copy,
    From,
    Into,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Id(u32);

#[derive(
    new,
    Default,
    Clone,
    Copy,
    From,
    Into,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Version(u16);

impl Version {
    pub(crate) const CURRENT: Version = Version(0);
}

pub type Payload<'m> = Cow<'m, [u8]>;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct MagicCookie;

impl MagicCookie {
    const VALUE: u32 = 0x42adde42;
}

impl std::fmt::Display for MagicCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", Self::VALUE)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{from_bytes, to_bytes};
    use serde_test::{assert_tokens, Token};

    pub fn samples() -> [Message; 3] {
        use subject::*;
        [
            Message {
                id: 123,
                version: Message::CURRENT_VERSION,
                kind: Kind::Post,
                flags: Flags::RETURN_TYPE,
                subject: Subject::try_from_values(
                    Service::Other(543.into()),
                    Object::Other(32.into()),
                    action::BoundObject::Terminate,
                )
                .unwrap(),
                payload: vec![1, 2, 3],
            },
            Message {
                id: 9034,
                version: Message::CURRENT_VERSION,
                kind: Kind::Event,
                flags: Flags::empty(),
                subject: Subject::try_from_values(
                    Service::Other(90934.into()),
                    Object::Other(178.into()),
                    action::BoundObject::Metaobject,
                )
                .unwrap(),
                payload: vec![],
            },
            Message {
                id: 21932,
                version: Message::CURRENT_VERSION,
                kind: Kind::Capability,
                flags: Flags::DYNAMIC_PAYLOAD,
                subject: ServiceDirectory {
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
    fn test_annotated_value_from_message() {
        let input = vec![
            0x42, 0xde, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            // payload
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let message: Message = from_reader(input.as_slice()).unwrap();
        assert_eq!(
            message,
            Message {
                id: 990340,
                version: 0,
                kind: Kind::Error,
                flags: Flags::empty(),
                subject: Subject::try_from_values(
                    subject::service::Id(47),
                    subject::object::Id(1),
                    subject::action::Id(178)
                )
                .unwrap(),
                payload: vec![
                    0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20,
                    0x72, 0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20,
                    0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64
                ],
            }
        );
        let value: Value = from_message(&message).unwrap();
        assert_eq!(value, Value::from("The robot is not localized"));
    }

    #[test]
    fn test_message_to_bytes() {
        use crate::message::*;
        let msg = Message {
            id: 329,
            version: 12,
            kind: Kind::Capability,
            flags: Flags::RETURN_TYPE,
            subject: subject::ServiceDirectory {
                action: subject::action::ServiceDirectory::ServiceReady,
            }
            .into(),
            payload: vec![0x17, 0x2b, 0xe6, 0x01, 0x5f],
        };
        let buf = to_bytes(&msg).unwrap();
        assert_eq!(
            buf,
            vec![
                0x42, 0xde, 0xad, 0x42, // cookie
                0x49, 0x01, 0x00, 0x00, // id
                0x05, 0x00, 0x00, 0x00, // size
                0x0c, 0x00, 0x06, 0x02, // version, type, flags
                0x01, 0x00, 0x00, 0x00, // service
                0x01, 0x00, 0x00, 0x00, // object
                0x68, 0x00, 0x00, 0x00, // action
                0x17, 0x2b, 0xe6, 0x01, 0x5f, // payload
            ]
        );
    }

    #[test]
    fn test_subject_to_bytes() {
        let subject =
            BoundObject::from_values_unchecked(service::Id(23), object::Id(923), action::Id(392));
        let buf = to_bytes(&subject).unwrap();
        assert_eq!(
            buf,
            vec![
                0x17, 0x00, 0x00, 0x00, // service
                0x9b, 0x03, 0x00, 0x00, // object
                0x88, 0x01, 0x00, 0x00, // action
            ]
        );
    }

    #[test]
    fn test_message_from_bytes() {
        let input = &[
            0x42, 0xde, 0xad, 0x42, // cookie
            0xb8, 0x9a, 0x00, 0x00, // id
            0x28, 0x00, 0x00, 0x00, // size
            0xaa, 0x00, 0x02, 0x00, // version, type, flags
            0x27, 0x00, 0x00, 0x00, // service
            0x09, 0x00, 0x00, 0x00, // object
            0x68, 0x00, 0x00, 0x00, // action
            // payload
            0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d, 0x65,
            0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35, 0x32, 0x2d,
            0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30, 0x33,
            // garbage at the end, should be ignored
            0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
            0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
        ];
        let msg: Message = from_bytes(input).unwrap();
        use crate::message::{subject::*, *};
        assert_eq!(
            msg,
            Message {
                id: 39608,
                version: 170,
                kind: Kind::Reply,
                flags: Flags::empty(),
                subject: Subject::try_from_values(service::Id(39), object::Id(9), action::Id(104),)
                    .unwrap(),
                payload: vec![
                    0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d,
                    0x65, 0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35,
                    0x32, 0x2d, 0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30,
                    0x33,
                ],
            }
        );
    }

    #[test]
    fn test_message_from_bytes_bad_cookie() {
        let input = &[
            0x42, 0xdf, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72,
            0x6f, 0x62, 0x6f, 0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f,
            0x63, 0x61, 0x6c, 0x69, 0x7a, 0x65, 0x64,
        ];
        let msg: Result<Message> = from_bytes(input);
        assert_matches!(msg, Err(Error::Custom(err)) => {
            assert!(err.starts_with("invalid value"), "error does not start with \"invalid value\": \"{}\"", err);
            assert!(err.ends_with("0x42adde42"), "error does not end with magic cookie value: \"{}\"", err);
        });
    }

    #[test]
    fn test_message_from_bytes_bad_size() {
        let input = &[
            0x42, 0xde, 0xad, 0x42, // cookie,
            0x84, 0x1c, 0x0f, 0x00, // id
            0x23, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x03, 0x00, // version, type, flags
            0x2f, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0xb2, 0x00, 0x00, // action, 1 byte short
        ];
        let msg: Result<Message> = from_bytes(input);
        assert_matches!(msg, Err(Error::Io(_)));
    }
}
