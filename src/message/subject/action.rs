#[derive(
    Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct Id(pub u32);

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u32> for Id {
    fn from(i: u32) -> Self {
        Self(i)
    }
}

impl From<Id> for u32 {
    fn from(id: Id) -> Self {
        id.0
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, serde::Serialize)]
#[serde(into = "Id")]
pub enum Action {
    Server(Server),
    ServiceDirectory(ServiceDirectory),
    BoundObject(BoundObject),
}

impl Default for Action {
    fn default() -> Self {
        Self::Server(Server::default())
    }
}

impl From<Server> for Action {
    fn from(s: Server) -> Self {
        Self::Server(s)
    }
}

impl From<ServiceDirectory> for Action {
    fn from(sd: ServiceDirectory) -> Self {
        Self::ServiceDirectory(sd)
    }
}

impl From<BoundObject> for Action {
    fn from(b: BoundObject) -> Self {
        Self::BoundObject(b)
    }
}

impl From<Action> for Id {
    fn from(action: Action) -> Self {
        match action {
            Action::Server(s) => s.into(),
            Action::ServiceDirectory(sd) => sd.into(),
            Action::BoundObject(b) => b.into(),
        }
    }
}

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u32)]
pub enum Server {
    #[default]
    Connect,
    Authenticate,
}

impl Server {
    const ID_CONNECT: u32 = 4;
    const ID_AUTHENTICATE: u32 = 8;
}

impl std::convert::TryFrom<Id> for Server {
    type Error = ServerError;

    fn try_from(id: Id) -> Result<Self, Self::Error> {
        match id {
            Id(Self::ID_CONNECT) => Ok(Self::Connect),
            Id(Self::ID_AUTHENTICATE) => Ok(Self::Authenticate),
            _ => Err(ServerError(id)),
        }
    }
}

impl From<Server> for Id {
    fn from(s: Server) -> Self {
        match s {
            Server::Connect => Server::ID_CONNECT,
            Server::Authenticate => Server::ID_AUTHENTICATE,
        }
        .into()
    }
}

#[derive(thiserror::Error, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[error("invalid server action {0}")]
pub struct ServerError(pub Id);

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u32)]
pub enum ServiceDirectory {
    #[default]
    Service,
    Services,
    RegisterService,
    UnregisterService,
    ServiceReady,
    UpdateServiceInfo,
    ServiceAdded,
    ServiceRemoved,
    MachineId,
}

impl ServiceDirectory {
    const ID_SERVICE: u32 = 100;
    const ID_SERVICES: u32 = 101;
    const ID_REGISTER_SERVICE: u32 = 102;
    const ID_UNREGISTER_SERVICE: u32 = 103;
    const ID_SERVICE_READY: u32 = 104;
    const ID_UPDATE_SERVICE_INFO: u32 = 105;
    const ID_SERVICE_ADDED: u32 = 106;
    const ID_SERVICE_REMOVED: u32 = 107;
    const ID_MACHINE_ID: u32 = 108;
}

impl std::convert::TryFrom<Id> for ServiceDirectory {
    type Error = ServiceDirectoryError;

    fn try_from(id: Id) -> Result<Self, Self::Error> {
        match id {
            Id(Self::ID_SERVICE) => Ok(Self::Service),
            Id(Self::ID_SERVICES) => Ok(Self::Services),
            Id(Self::ID_REGISTER_SERVICE) => Ok(Self::RegisterService),
            Id(Self::ID_UNREGISTER_SERVICE) => Ok(Self::UnregisterService),
            Id(Self::ID_SERVICE_READY) => Ok(Self::ServiceReady),
            Id(Self::ID_UPDATE_SERVICE_INFO) => Ok(Self::UpdateServiceInfo),
            Id(Self::ID_SERVICE_ADDED) => Ok(Self::ServiceAdded),
            Id(Self::ID_SERVICE_REMOVED) => Ok(Self::ServiceRemoved),
            Id(Self::ID_MACHINE_ID) => Ok(Self::MachineId),
            _ => Err(ServiceDirectoryError(id)),
        }
    }
}

impl From<ServiceDirectory> for Id {
    fn from(sd: ServiceDirectory) -> Self {
        match sd {
            ServiceDirectory::Service => ServiceDirectory::ID_SERVICE,
            ServiceDirectory::Services => ServiceDirectory::ID_SERVICES,
            ServiceDirectory::RegisterService => ServiceDirectory::ID_REGISTER_SERVICE,
            ServiceDirectory::UnregisterService => ServiceDirectory::ID_UNREGISTER_SERVICE,
            ServiceDirectory::ServiceReady => ServiceDirectory::ID_SERVICE_READY,
            ServiceDirectory::UpdateServiceInfo => ServiceDirectory::ID_UPDATE_SERVICE_INFO,
            ServiceDirectory::ServiceAdded => ServiceDirectory::ID_SERVICE_ADDED,
            ServiceDirectory::ServiceRemoved => ServiceDirectory::ID_SERVICE_REMOVED,
            ServiceDirectory::MachineId => ServiceDirectory::ID_MACHINE_ID,
        }
        .into()
    }
}

#[derive(thiserror::Error, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[error("invalid service directory action {0}")]
pub struct ServiceDirectoryError(pub Id);

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u32)]
pub enum BoundObject {
    #[default]
    RegisterEvent,
    UnregisterEvent,
    Metaobject,
    Terminate,
    Property,
    SetProperty,
    Properties,
    RegisterEventWithSignature,
    BoundFunction(Id),
}

impl BoundObject {
    const ID_REGISTER_EVENT: u32 = 0;
    const ID_UNREGISTER_EVENT: u32 = 1;
    const ID_METAOBJECT: u32 = 2;
    const ID_TERMINATE: u32 = 3;
    const ID_PROPERTY: u32 = 5; // not a typo, there is no action 4
    const ID_SET_PROPERTY: u32 = 6;
    const ID_PROPERTIES: u32 = 7;
    const ID_REGISTER_EVENT_WITH_SIGNATURE: u32 = 8;
}

impl From<Id> for BoundObject {
    fn from(id: Id) -> Self {
        match id {
            Id(Self::ID_REGISTER_EVENT) => Self::RegisterEvent,
            Id(Self::ID_UNREGISTER_EVENT) => Self::UnregisterEvent,
            Id(Self::ID_METAOBJECT) => Self::Metaobject,
            Id(Self::ID_TERMINATE) => Self::Terminate,
            Id(Self::ID_PROPERTY) => Self::Property,
            Id(Self::ID_SET_PROPERTY) => Self::SetProperty,
            Id(Self::ID_PROPERTIES) => Self::Properties,
            Id(Self::ID_REGISTER_EVENT_WITH_SIGNATURE) => Self::RegisterEventWithSignature,
            id => Self::BoundFunction(id),
        }
    }
}

impl From<BoundObject> for Id {
    fn from(bo: BoundObject) -> Self {
        match bo {
            BoundObject::RegisterEvent => BoundObject::ID_REGISTER_EVENT.into(),
            BoundObject::UnregisterEvent => BoundObject::ID_UNREGISTER_EVENT.into(),
            BoundObject::Metaobject => BoundObject::ID_METAOBJECT.into(),
            BoundObject::Terminate => BoundObject::ID_TERMINATE.into(),
            BoundObject::Property => BoundObject::ID_PROPERTY.into(),
            BoundObject::SetProperty => BoundObject::ID_SET_PROPERTY.into(),
            BoundObject::Properties => BoundObject::ID_PROPERTIES.into(),
            BoundObject::RegisterEventWithSignature => {
                BoundObject::ID_REGISTER_EVENT_WITH_SIGNATURE.into()
            }
            BoundObject::BoundFunction(id) => id,
        }
    }
}
