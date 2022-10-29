pub mod kind;
pub use kind::Kind;

pub mod flags;
pub use flags::Flags;

pub mod action;
pub use action::Action;

pub mod subject;
pub use subject::Subject;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Message {
    pub id: u32,
    pub version: u16,
    pub kind: Kind,
    pub flags: Flags,
    pub subject: Subject,
    pub payload: Vec<u8>,
}

impl Message {
    pub const CURRENT_VERSION: u16 = 0;
    pub const TOKEN: &'static str = "qi.Message";
    pub const ID_TOKEN: &'static str = "id";
    pub const VERSION_TOKEN: &'static str = "version";
    pub const KIND_TOKEN: &'static str = "type";
    pub const FLAGS_TOKEN: &'static str = "flags";
    pub const SUBJECT_TOKEN: &'static str = "subject";
    pub const PAYLOAD_TOKEN: &'static str = "payload";
    pub const FIELDS: &[&'static str; 6] = &[
        Message::ID_TOKEN,
        Message::VERSION_TOKEN,
        Message::KIND_TOKEN,
        Message::FLAGS_TOKEN,
        Message::SUBJECT_TOKEN,
        Message::PAYLOAD_TOKEN,
    ];

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

impl serde::Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_struct(Self::TOKEN, 6)?;
        use serde::ser::SerializeStruct;
        ser.serialize_field(Self::ID_TOKEN, &self.id)?;
        ser.serialize_field(Self::VERSION_TOKEN, &self.version)?;
        ser.serialize_field(Self::KIND_TOKEN, &self.kind)?;
        ser.serialize_field(Self::FLAGS_TOKEN, &self.flags)?;
        ser.serialize_field(Self::SUBJECT_TOKEN, &self.subject)?;
        ser.serialize_field(Self::PAYLOAD_TOKEN, serde_bytes::Bytes::new(&self.payload))?;
        ser.end()
    }
}

impl<'de> serde::Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum Field {
            Id,
            Version,
            Kind,
            Flags,
            Subject,
            Payload,
        }

        impl<'de> de::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                struct FieldVisitor;
                impl<'de> de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        write!(formatter, "any of {}", Message::FIELDS.join(" "))
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            Message::ID_TOKEN => Ok(Field::Id),
                            Message::VERSION_TOKEN => Ok(Field::Version),
                            Message::KIND_TOKEN => Ok(Field::Kind),
                            Message::FLAGS_TOKEN => Ok(Field::Flags),
                            Message::SUBJECT_TOKEN => Ok(Field::Subject),
                            Message::PAYLOAD_TOKEN => Ok(Field::Payload),
                            _ => Err(de::Error::unknown_field(value, Message::FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct Visitor;
        use serde::de;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Message;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "struct {}", Message::TOKEN)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let map = &mut map;
                let mut id = None;
                let mut version = None;
                let mut kind = None;
                let mut flags = None;
                let mut subject = None;
                let mut payload: Option<serde_bytes::ByteBuf> = None;
                fn set_next_value<'de, A, T>(
                    value: &mut Option<T>,
                    map: &mut A,
                    field: &'static str,
                ) -> Result<(), <A as de::MapAccess<'de>>::Error>
                where
                    A: de::MapAccess<'de>,
                    T: de::Deserialize<'de>,
                {
                    match value {
                        None => {
                            *value = Some(map.next_value()?);
                            Ok(())
                        }
                        Some(_) => return Err(de::Error::duplicate_field(field)),
                    }
                }

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => set_next_value(&mut id, map, Message::ID_TOKEN)?,
                        Field::Version => {
                            set_next_value(&mut version, map, Message::VERSION_TOKEN)?
                        }
                        Field::Kind => set_next_value(&mut kind, map, Message::KIND_TOKEN)?,
                        Field::Flags => set_next_value(&mut flags, map, Message::FLAGS_TOKEN)?,
                        Field::Subject => {
                            set_next_value(&mut subject, map, Message::SUBJECT_TOKEN)?
                        }
                        Field::Payload => {
                            set_next_value(&mut payload, map, Message::PAYLOAD_TOKEN)?
                        }
                    }
                }
                let missing_field = |field| move || de::Error::missing_field(field);
                let id = id.ok_or_else(missing_field(Message::ID_TOKEN))?;
                let version = version.ok_or_else(missing_field(Message::VERSION_TOKEN))?;
                let kind = kind.ok_or_else(missing_field(Message::KIND_TOKEN))?;
                let flags = flags.ok_or_else(missing_field(Message::FLAGS_TOKEN))?;
                let subject = subject.ok_or_else(missing_field(Message::SUBJECT_TOKEN))?;
                let payload = payload
                    .ok_or_else(missing_field(Message::PAYLOAD_TOKEN))?
                    .into_vec();
                Ok(Message {
                    id,
                    version,
                    kind,
                    flags,
                    subject,
                    payload,
                })
            }
        }

        deserializer.deserialize_struct(Self::TOKEN, Self::FIELDS, Visitor)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct MagicCookie;

impl MagicCookie {
    pub const VALUE: u32 = 0x42adde42;
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
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Self::VALUE.serialize(serializer)
    }
}

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
                &MagicCookie,
            ));
        }
        Ok(MagicCookie)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
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
}
