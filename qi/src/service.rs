use crate::{object, os, session};
pub use qi_value::ServiceId as Id;

pub(crate) const MAIN_OBJECT_ID: object::Id = object::Id(1);

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "crate::value", rename_all = "camelCase")]
pub struct Info {
    name: String,
    service_id: Id,
    machine_id: os::MachineId,
    process_id: u32,
    endpoints: Vec<session::Reference>,
    session_id: session::Uid,
    object_uid: object::Uid,
}

impl Info {
    pub(crate) fn process_local(
        name: String,
        service_id: Id,
        endpoints: Vec<session::Reference>,
        session_id: session::Uid,
        object_uid: object::Uid,
    ) -> Self {
        Self {
            name,
            service_id,
            machine_id: os::MachineId::local(),
            process_id: std::process::id(),
            endpoints,
            session_id,
            object_uid,
        }
    }

    pub fn id(&self) -> Id {
        self.service_id
    }

    pub fn machine_id(&self) -> os::MachineId {
        self.machine_id
    }

    pub fn process_id(&self) -> u32 {
        self.process_id
    }

    pub fn endpoints(&self) -> &[session::Reference] {
        &self.endpoints
    }

    pub fn session_uid(&self) -> session::Uid {
        self.session_id.clone()
    }

    pub fn object_uid(&self) -> object::Uid {
        self.object_uid
    }
}

impl std::fmt::Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Info {
            name,
            service_id,
            machine_id,
            process_id,
            endpoints,
            session_id,
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
            "], session={session_id}, \
                object={object_uid})"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{binary_value, messaging};
    use messaging::Address;
    use std::net::{Ipv4Addr, SocketAddr};

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
        let service_info: Info = binary_value::deserialize_reflect(&mut input).unwrap();
        assert_eq!(
            service_info,
            Info {
                name: "Calculator".to_owned(),
                service_id: Id(2),
                machine_id: "9a65b56e-c3d3-4485-8924-661b036202b3".parse().unwrap(),
                process_id: 3420486,
                endpoints: vec![
                    session::Reference::service("Calculator"),
                    session::Reference::endpoint(Address::Tcp {
                        address: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 41681),
                        ssl: None
                    })
                ],
                session_id: session::Uid::from_string(
                    "361ecec4-00f7-4c94-a6e2-d91e28c5a06c".to_owned()
                ),
                object_uid: object::Uid::from_bytes([
                    0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33,
                    0x3d, 0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16
                ])
            }
        )
    }

    #[test]
    fn test_object_uid_deserialize() {
        let mut input = &[
            0x14, 0x00, 0x00, 0x00, 0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42,
            0x20, 0xb7, 0x33, 0x3d, 0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16,
        ][..];
        let object_uid: object::Uid = binary_value::deserialize_reflect(&mut input).unwrap();
        assert_eq!(
            object_uid,
            object::Uid::from_bytes([
                0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33, 0x3d,
                0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16
            ])
        );
    }
}
