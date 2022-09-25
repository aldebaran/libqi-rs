pub mod message {
    use bitflags::bitflags;
    use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
    use num_derive::{FromPrimitive, ToPrimitive};
    use num_traits::{FromPrimitive, ToPrimitive};
    use std::{
        io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write},
        ops::RangeInclusive,
    };

    #[derive(
        FromPrimitive, ToPrimitive, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy,
    )]
    enum Kind {
        None,       // 0
        Call,       // 1
        Reply,      // 2
        Error,      // 3
        Post,       // 4
        Event,      // 5
        Capability, // 6
        Cancel,     // 7
        Canceled,   // 8
    }

    impl Kind {
        fn write<W>(&self, writer: &mut W) -> IoResult<()>
        where
            W: Write,
        {
            writer.write_u8(self.to_u8().unwrap())
        }
    }

    impl Default for Kind {
        fn default() -> Self {
            Self::None
        }
    }

    bitflags! {
        #[derive(Default)]
        struct Flags: u8 {
            const DYNAMIC_PAYLOAD = 0b00000001;
            const RETURN_TYPE = 0b00000010;
        }
    }

    impl Flags {
        fn write<W>(&self, writer: &mut W) -> IoResult<()>
        where
            W: Write,
        {
            writer.write_u8(self.bits())
        }
    }

    #[derive(
        FromPrimitive, ToPrimitive, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy,
    )]
    enum ServerAction {
        Connect = 4,
        Authenticate = 8,
    }

    impl Default for ServerAction {
        fn default() -> Self {
            Self::Connect
        }
    }

    const ACTION_ID_SD_SERVICE: u32 = 100;
    const ACTION_ID_SD_SERVICES: u32 = 101;
    const ACTION_ID_SD_REGISTER_SERVICE: u32 = 102;
    const ACTION_ID_SD_UNREGISTER_SERVICE: u32 = 103;
    const ACTION_ID_SD_SERVICE_READY: u32 = 104;
    const ACTION_ID_SD_UPDATE_SERVICE_INFO: u32 = 105;
    const ACTION_ID_SD_SERVICE_ADDED: u32 = 106;
    const ACTION_ID_SD_SERVICE_REMOVED: u32 = 107;
    const ACTION_ID_SD_MACHINE_ID: u32 = 108;

    #[derive(
        FromPrimitive, ToPrimitive, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy,
    )]
    enum ServiceDirectoryAction {
        Service = ACTION_ID_SD_SERVICE as isize,
        Services = ACTION_ID_SD_SERVICES as isize,
        RegisterService = ACTION_ID_SD_REGISTER_SERVICE as isize,
        UnregisterService = ACTION_ID_SD_UNREGISTER_SERVICE as isize,
        ServiceReady = ACTION_ID_SD_SERVICE_READY as isize,
        UpdateServiceInfo = ACTION_ID_SD_UPDATE_SERVICE_INFO as isize,
        ServiceAdded = ACTION_ID_SD_SERVICE_ADDED as isize,
        ServiceRemoved = ACTION_ID_SD_SERVICE_REMOVED as isize,
        MachineId = ACTION_ID_SD_MACHINE_ID as isize,
    }

    impl Default for ServiceDirectoryAction {
        fn default() -> Self {
            Self::Service
        }
    }

    const ACTION_ID_REGISTER_EVENT: u32 = 0;
    const ACTION_ID_UNREGISTER_EVENT: u32 = 1;
    const ACTION_ID_METAOBJECT: u32 = 2;
    const ACTION_ID_TERMINATE: u32 = 3;
    const ACTION_ID_PROPERTY: u32 = 5; // not a typo, there is no action 4
    const ACTION_ID_SET_PROPERTY: u32 = 6;
    const ACTION_ID_PROPERTIES: u32 = 7;
    const ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE: u32 = 8;

    #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    enum BoundObjectAction {
        RegisterEvent,
        UnregisterEvent,
        Metaobject,
        Terminate,
        Property,
        SetProperty,
        Properties,
        RegisterEventWithSignature,
        BoundFunction(u32),
    }

    impl Default for BoundObjectAction {
        fn default() -> Self {
            Self::RegisterEvent
        }
    }

    impl FromPrimitive for BoundObjectAction {
        fn from_u32(n: u32) -> Option<Self> {
            Some(match n {
                ACTION_ID_REGISTER_EVENT => Self::RegisterEvent,
                ACTION_ID_UNREGISTER_EVENT => Self::UnregisterEvent,
                ACTION_ID_METAOBJECT => Self::Metaobject,
                ACTION_ID_TERMINATE => Self::Terminate,
                ACTION_ID_PROPERTY => Self::Property,
                ACTION_ID_SET_PROPERTY => Self::SetProperty,
                ACTION_ID_PROPERTIES => Self::Properties,
                ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE => Self::RegisterEventWithSignature,
                _ => Self::BoundFunction(n),
            })
        }

        fn from_i64(n: i64) -> Option<Self> {
            Self::from_u32(n.try_into().ok()?)
        }

        fn from_u64(n: u64) -> Option<Self> {
            Self::from_u32(n.try_into().ok()?)
        }
    }

    impl ToPrimitive for BoundObjectAction {
        fn to_u32(&self) -> Option<u32> {
            Some(match self {
                BoundObjectAction::RegisterEvent => ACTION_ID_REGISTER_EVENT,
                BoundObjectAction::UnregisterEvent => ACTION_ID_UNREGISTER_EVENT,
                BoundObjectAction::Metaobject => ACTION_ID_METAOBJECT,
                BoundObjectAction::Terminate => ACTION_ID_TERMINATE,
                BoundObjectAction::Property => ACTION_ID_PROPERTY,
                BoundObjectAction::SetProperty => ACTION_ID_SET_PROPERTY,
                BoundObjectAction::Properties => ACTION_ID_PROPERTIES,
                BoundObjectAction::RegisterEventWithSignature => {
                    ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE
                }
                BoundObjectAction::BoundFunction(n) => *n,
            })
        }

        fn to_i64(&self) -> Option<i64> {
            Some(self.to_u32().unwrap().into())
        }

        fn to_u64(&self) -> Option<u64> {
            Some(self.to_u32().unwrap().into())
        }
    }

    const SERVICE_ID_SERVER: u32 = 0;
    const SERVICE_ID_SERVICE_DIRECTORY: u32 = 1;

    const OBJECT_ID_NONE: u32 = 0;
    const OBJECT_ID_SERVICE_MAIN: u32 = 1;

    #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    enum Target {
        // service = server, object = none
        Server(ServerAction),
        // service = sd, object = service main
        ServiceDirectory(ServiceDirectoryAction),
        // other
        BoundObject {
            service: u32,
            object: u32,
            action: BoundObjectAction,
        },
    }

    impl Target {
        fn from_values(service: u32, object: u32, action: u32) -> Option<Self> {
            match (service, object, action) {
                (SERVICE_ID_SERVER, OBJECT_ID_NONE, action) => {
                    Some(Self::Server(ServerAction::from_u32(action)?))
                }
                (SERVICE_ID_SERVICE_DIRECTORY, OBJECT_ID_SERVICE_MAIN, action) => Some(
                    Self::ServiceDirectory(ServiceDirectoryAction::from_u32(action)?),
                ),
                (service, object, action)
                    if service != SERVICE_ID_SERVER && object != OBJECT_ID_NONE =>
                {
                    Some(Self::BoundObject {
                        service,
                        object,
                        action: BoundObjectAction::from_u32(action).unwrap(),
                    })
                }
                _ => None,
            }
        }

        fn service(&self) -> u32 {
            match self {
                Self::Server(_) => SERVICE_ID_SERVER,
                Self::ServiceDirectory(_) => SERVICE_ID_SERVICE_DIRECTORY,
                Self::BoundObject { service, .. } => *service,
            }
        }

        fn object(&self) -> u32 {
            match self {
                Self::Server(_) => OBJECT_ID_NONE,
                Self::ServiceDirectory(_) => OBJECT_ID_SERVICE_MAIN,
                Self::BoundObject { object, .. } => *object,
            }
        }

        fn action(&self) -> u32 {
            match self {
                Self::Server(act) => act.to_u32(),
                Self::ServiceDirectory(act) => act.to_u32(),
                Self::BoundObject { action, .. } => action.to_u32(),
            }
            .unwrap()
        }

        fn write<W>(&self, writer: &mut W) -> IoResult<()>
        where
            W: Write,
        {
            writer.write_u32::<LittleEndian>(self.service())?;
            writer.write_u32::<LittleEndian>(self.object())?;
            writer.write_u32::<LittleEndian>(self.action())
        }
    }

    #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
    pub struct Message {
        id: u32,
        kind: Kind, // or type
        flags: Flags,
        target: Target,
        payload: Vec<u8>,
    }

    impl Message {
        const VERSION: u16 = 0;
        const MAGIC_COOKIE: u32 = 0x42dead42;

        pub fn write<W>(&self, writer: &mut W) -> IoResult<()>
        where
            W: Write,
        {
            let payload_size = self.payload.len();
            let payload_size_u32 = match payload_size.try_into() {
                Ok(size) => size,
                Err(err) => {
                    return Err(IoError::new(
                        IoErrorKind::Other,
                        format!("bad payload size: {err}"),
                    ))
                }
            };

            writer.write_u32::<BigEndian>(Self::MAGIC_COOKIE)?;
            writer.write_u32::<LittleEndian>(self.id)?;
            writer.write_u32::<LittleEndian>(payload_size_u32)?;
            writer.write_u16::<LittleEndian>(Self::VERSION)?;
            self.kind.write(writer)?;
            self.flags.write(writer)?;
            self.target.write(writer)?;
            writer.write_all(&self.payload)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn message_write() -> IoResult<()> {
            let msg = Message {
                id: 329,
                kind: Kind::Capability,
                flags: Flags::RETURN_TYPE,
                target: Target::ServiceDirectory(ServiceDirectoryAction::ServiceReady),
                payload: vec![23u8, 43u8, 230u8, 1u8, 95u8],
            };
            let mut buf = Vec::new();
            msg.write(&mut buf)?;
            let expected = vec![
                0x42, 0xde, 0xad, 0x42, // cookie
                0x49, 0x01, 0x00, 0x00, // id
                0x05, 0x00, 0x00, 0x00, // size
                0x00, 0x00, 0x06, 0x02, // version, type, flags
                0x01, 0x00, 0x00, 0x00, // service
                0x01, 0x00, 0x00, 0x00, // object
                0x68, 0x00, 0x00, 0x00, // action
                0x17, 0x2b, 0xe6, 0x01, 0x5f, // payload
            ];
            assert_eq!(buf, expected);
            Ok(())
        }
    }
}

use message::Message;
