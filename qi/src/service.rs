use crate::{
    node, object, session,
    value::{self, os},
};
use qi_value::RuntimeReflect;
pub use value::ServiceId as Id;

pub(super) const MAIN_OBJECT_ID: object::Id = object::Id(1);
pub(super) const UNSPECIFIED_ID: Id = Id(0);

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    qi_macros::Valuable,
)]
#[qi(value(crate = "crate::value", case = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub(super) name: String,
    #[qi(value(name = "serviceId"))]
    #[serde(alias = "serviceId")]
    pub(super) id: Id,
    pub(super) machine_id: os::MachineId,
    pub(super) process_id: u32,
    pub(super) endpoints: Vec<session::Target>,
    #[qi(value(name = "sessionId"))]
    #[serde(alias = "sessionId")] // "Session" is the legacy name for nodes.
    pub(super) node_uid: node::Uid,
    /// Object uid in service info are represented as strings containing pure binary data for
    /// compatibility reasons. They are therefore NOT UTF-8 valid strings or even contain printable
    /// characters.
    pub(super) object_uid: ObjectUidAsStr,
}

impl Info {
    pub(super) fn process_local(
        name: String,
        id: Id,
        endpoints: Vec<session::Target>,
        node_uid: node::Uid,
        object_uid: object::Uid,
    ) -> Self {
        Self {
            name,
            id,
            machine_id: os::MachineId::local(),
            process_id: std::process::id(),
            endpoints,
            node_uid,
            object_uid: ObjectUidAsStr(object_uid),
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn machine_id(&self) -> os::MachineId {
        self.machine_id
    }

    pub fn process_id(&self) -> u32 {
        self.process_id
    }

    pub fn endpoints(&self) -> &[session::Target] {
        &self.endpoints
    }

    pub fn node_uid(&self) -> node::Uid {
        self.node_uid.clone()
    }

    pub fn object_uid(&self) -> object::Uid {
        self.object_uid.0
    }
}

impl std::fmt::Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Info {
            name,
            id: service_id,
            machine_id,
            process_id,
            endpoints,
            node_uid,
            object_uid,
        } = self;
        write!(
            f,
            "{name}({service_id}, machine={machine_id}, \
                process={process_id}, \
                endpoints=["
        )?;
        for endpoint in endpoints {
            endpoint.fmt(f)?;
        }
        write!(
            f,
            "], node={node_uid}, \
                object={object_uid})"
        )
    }
}

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
pub struct ObjectUidAsStr(pub object::Uid);

impl value::Reflect for ObjectUidAsStr {
    fn ty() -> Option<value::Type> {
        Some(value::Type::String)
    }
}

impl value::RuntimeReflect for ObjectUidAsStr {
    fn ty(&self) -> value::Type {
        value::Type::String
    }
}

impl value::ToValue for ObjectUidAsStr {
    fn to_value(&self) -> value::Value<'_> {
        value::String::from_maybe_utf8(self.0.bytes()).into()
    }
}

impl<'a> value::IntoValue<'a> for ObjectUidAsStr {
    fn into_value(self) -> value::Value<'a> {
        value::String::from_maybe_utf8_owned(self.0.bytes().to_vec()).into()
    }
}

impl<'a> value::FromValue<'a> for ObjectUidAsStr {
    fn from_value(value: value::Value<'a>) -> std::result::Result<Self, value::FromValueError> {
        let value_type = value.ty();
        let value_str = value
            .into_string()
            .ok_or_else(|| value::FromValueError::TypeMismatch {
                expected: "an Object UID".to_owned(),
                actual: value_type.to_string(),
            })?;
        let bytes = <[u8; 20]>::try_from(value_str.as_bytes())
            .map_err(|err| value::FromValueError::Other(err.into()))?;
        Ok(Self(bytes.into()))
    }
}

impl serde::Serialize for ObjectUidAsStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_with::As::<serde_with::Bytes>::serialize(self.0.bytes(), serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ObjectUidAsStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = serde_with::As::<serde_with::Bytes>::deserialize(deserializer)?;
        Ok(Self(object::Uid::from_bytes(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging;
    use messaging::Address;
    use qi_format::{from_slice, to_bytes};
    use qi_value::{de::ValueType, Reflect};
    use serde::de::DeserializeSeed;
    use std::net::{Ipv4Addr, SocketAddr};

    #[test]
    fn service_info_from_format_value() {
        let value_in = &[
            0x0a, 0x00, 0x00, 0x00, 0x43, 0x61, 0x6c, 0x63, 0x75, 0x6c, 0x61, 0x74, 0x6f, 0x72,
            0x02, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x39, 0x61, 0x36, 0x35, 0x62, 0x35,
            0x36, 0x65, 0x2d, 0x63, 0x33, 0x64, 0x33, 0x2d, 0x34, 0x34, 0x38, 0x35, 0x2d, 0x38,
            0x39, 0x32, 0x34, 0x2d, 0x36, 0x36, 0x31, 0x62, 0x30, 0x33, 0x36, 0x32, 0x30, 0x32,
            0x62, 0x33, 0x46, 0x31, 0x34, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00,
            0x71, 0x69, 0x3a, 0x43, 0x61, 0x6c, 0x63, 0x75, 0x6c, 0x61, 0x74, 0x6f, 0x72, 0x15,
            0x00, 0x00, 0x00, 0x74, 0x63, 0x70, 0x3a, 0x2f, 0x2f, 0x31, 0x32, 0x37, 0x2e, 0x30,
            0x2e, 0x30, 0x2e, 0x31, 0x3a, 0x34, 0x31, 0x36, 0x38, 0x31, 0x24, 0x00, 0x00, 0x00,
            0x33, 0x36, 0x31, 0x65, 0x63, 0x65, 0x63, 0x34, 0x2d, 0x30, 0x30, 0x66, 0x37, 0x2d,
            0x34, 0x63, 0x39, 0x34, 0x2d, 0x61, 0x36, 0x65, 0x32, 0x2d, 0x64, 0x39, 0x31, 0x65,
            0x32, 0x38, 0x63, 0x35, 0x61, 0x30, 0x36, 0x63, 0x14, 0x00, 0x00, 0x00, 0xfd, 0xeb,
            0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33, 0x3d, 0xc4, 0xe1,
            0x0d, 0x8a, 0xd6, 0x16,
        ][..];
        let service_info: Info = ValueType(<Info as Reflect>::ty().as_ref())
            .deserialize(qi_format::SliceDeserializer::new(value_in))
            .unwrap()
            .cast_into()
            .unwrap();
        assert_eq!(
            service_info,
            Info {
                name: "Calculator".to_owned(),
                id: Id(2),
                machine_id: "9a65b56e-c3d3-4485-8924-661b036202b3".parse().unwrap(),
                process_id: 3420486,
                endpoints: vec![
                    session::Target::service("Calculator"),
                    session::Target::from(Address::Tcp {
                        address: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 41681),
                        ssl: None
                    })
                ],
                node_uid: node::Uid::from_string("361ecec4-00f7-4c94-a6e2-d91e28c5a06c".to_owned()),
                object_uid: ObjectUidAsStr(object::Uid::from([
                    0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33,
                    0x3d, 0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16
                ]))
            }
        )
    }

    #[test]
    fn object_uid_from_to_format() {
        let value_in = &[
            0x14, 0x00, 0x00, 0x00, 0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42,
            0x20, 0xb7, 0x33, 0x3d, 0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16,
        ][..];
        let object_uid: ObjectUidAsStr = from_slice(value_in).unwrap();
        assert_eq!(
            object_uid.0,
            [
                0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33, 0x3d,
                0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16
            ]
        );
        let value_out = to_bytes(&object_uid).unwrap();
        assert_eq!(value_out, value_in);
    }
}
