use crate::{session, MachineId};
use qi_value::{self as value, ObjectId};
use std::borrow::Cow;
use value::ServiceId;

pub const MAIN_OBJECT_ID: ObjectId = ObjectId(1);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "qi_value", rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: String,
    pub service_id: ServiceId,
    pub machine_id: MachineId,
    pub process_id: u32,
    pub endpoints: Vec<session::Address>,
    pub session_id: SessionId,
    pub object_uid: ObjectUid,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "qi_value", transparent)]
pub struct SessionId(String);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ObjectUid(value::object::ObjectUid);

impl value::Reflect for ObjectUid {
    fn ty() -> Option<value::Type> {
        Some(value::Type::String)
    }
}

impl value::RuntimeReflect for ObjectUid {
    fn ty(&self) -> value::Type {
        value::Type::String
    }
}

impl value::ToValue for ObjectUid {
    fn to_value(&self) -> value::Value<'_> {
        value::Value::ByteString(Cow::Borrowed(self.0.bytes()))
    }
}

impl<'a> value::IntoValue<'a> for ObjectUid {
    fn into_value(self) -> value::Value<'a> {
        value::Value::ByteString(Cow::Owned(self.0.bytes().to_vec()))
    }
}

impl<'a> value::FromValue<'a> for ObjectUid {
    fn from_value(value: value::Value<'a>) -> Result<Self, value::FromValueError> {
        let bytes = value
            .as_string_bytes()
            .ok_or_else(|| value::FromValueError::TypeMismatch {
                expected: "an Object UID".to_owned(),
                actual: value.to_string(),
            })?;
        let bytes =
            <[u8; 20]>::try_from(bytes).map_err(|err| value::FromValueError::Other(err.into()))?;
        Ok(Self(value::object::ObjectUid::from_bytes(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node;
    use qi_format::de::BufExt;

    #[test]
    fn test_service_info_deserialize() {
        let mut input = &[
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
        let service_info: ServiceInfo = input.deserialize_value().unwrap();
        assert_eq!(
            service_info,
            ServiceInfo {
                name: "Calculator".to_owned(),
                service_id: ServiceId(2),
                machine_id: MachineId::new("9a65b56e-c3d3-4485-8924-661b036202b3".to_owned()),
                process_id: 3420486,
                endpoints: vec![
                    session::Address::Relative {
                        service: "Calculator".to_owned()
                    },
                    session::Address::Node(node::Address::Tcp {
                        host: "127.0.0.1".to_owned(),
                        port: 41681,
                        ssl: None
                    })
                ],
                session_id: SessionId("361ecec4-00f7-4c94-a6e2-d91e28c5a06c".to_owned()),
                object_uid: ObjectUid(value::object::ObjectUid::from_bytes([
                    0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33,
                    0x3d, 0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16
                ]))
            }
        )
    }

    #[test]
    fn test_object_uid_deserialize() {
        let mut input = &[
            0x14, 0x00, 0x00, 0x00, 0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42,
            0x20, 0xb7, 0x33, 0x3d, 0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16,
        ][..];
        let object_uid: ObjectUid = input.deserialize_value().unwrap();
        assert_eq!(
            object_uid,
            ObjectUid(value::object::ObjectUid::from_bytes([
                0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33, 0x3d,
                0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16
            ]))
        );
    }
}
