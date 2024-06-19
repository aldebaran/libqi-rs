use crate::{node, object, os, session};
pub use qi_value::ServiceId as Id;

pub(super) const MAIN_OBJECT_ID: object::Id = object::Id(1);

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value(crate = "crate::value", case = "camelCase"))]
pub struct Info {
    pub(super) name: String,
    #[qi(value(name = "serviceId"))]
    pub(super) id: Id,
    pub(super) machine_id: os::MachineId,
    pub(super) process_id: u32,
    pub(super) endpoints: Vec<session::Reference>,
    #[qi(value(name = "sessionId"))] // "Session" is the legacy name for nodes.
    pub(super) node_uid: node::Uid,
    pub(super) object_uid: object::Uid,
}

impl Info {
    pub(super) fn registrable(name: String, node_uid: node::Uid, object_uid: object::Uid) -> Self {
        Self::process_local(name, Id(0), Vec::new(), node_uid, object_uid)
    }
    pub(super) fn process_local(
        name: String,
        id: Id,
        endpoints: Vec<session::Reference>,
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
            object_uid,
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

    pub fn endpoints(&self) -> &[session::Reference] {
        &self.endpoints
    }

    pub fn node_uid(&self) -> node::Uid {
        self.node_uid.clone()
    }

    pub fn object_uid(&self) -> object::Uid {
        self.object_uid
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{messaging, value::BinaryValue};
    use messaging::Address;
    use std::net::{Ipv4Addr, SocketAddr};

    #[test]
    fn service_info_from_binary_value() {
        let mut binvalue = BinaryValue::from_static(&[
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
        ]);
        let service_info: Info = binvalue.deserialize_value().unwrap();
        assert_eq!(
            service_info,
            Info {
                name: "Calculator".to_owned(),
                id: Id(2),
                machine_id: "9a65b56e-c3d3-4485-8924-661b036202b3".parse().unwrap(),
                process_id: 3420486,
                endpoints: vec![
                    session::Reference::service("Calculator"),
                    session::Reference::endpoint(Address::Tcp {
                        address: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 41681),
                        ssl: None
                    })
                ],
                node_uid: node::Uid::from_string("361ecec4-00f7-4c94-a6e2-d91e28c5a06c".to_owned()),
                object_uid: object::Uid::from_bytes([
                    0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33,
                    0x3d, 0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16
                ])
            }
        )
    }

    #[test]
    fn object_uid_from_binary_value() {
        let mut binvalue = BinaryValue::from_static(&[
            0x14, 0x00, 0x00, 0x00, 0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42,
            0x20, 0xb7, 0x33, 0x3d, 0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16,
        ]);
        let object_uid: object::Uid = binvalue.deserialize_value().unwrap();
        assert_eq!(
            object_uid,
            object::Uid::from_bytes([
                0xfd, 0xeb, 0xc1, 0x2e, 0xcb, 0xea, 0x6b, 0x58, 0xcc, 0x42, 0x20, 0xb7, 0x33, 0x3d,
                0xc4, 0xe1, 0x0d, 0x8a, 0xd6, 0x16
            ])
        );
    }
}
