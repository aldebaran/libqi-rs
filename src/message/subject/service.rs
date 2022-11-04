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

#[derive(
    Default,
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
#[serde(into = "Id")]
#[serde(from = "Id")]
#[repr(u32)]
pub enum Service {
    #[default]
    Server,
    ServiceDirectory,
    Other(Id),
}

impl Service {
    const ID_SERVER: u32 = 0;
    const ID_SERVICE_DIRECTORY: u32 = 1;
}

impl From<Id> for Service {
    fn from(id: Id) -> Self {
        match id {
            Id(Self::ID_SERVER) => Self::Server,
            Id(Self::ID_SERVICE_DIRECTORY) => Self::ServiceDirectory,
            id => Self::Other(id),
        }
    }
}

impl From<Service> for Id {
    fn from(s: Service) -> Self {
        match s {
            Service::Server => Service::ID_SERVER.into(),
            Service::ServiceDirectory => Service::ID_SERVICE_DIRECTORY.into(),
            Service::Other(id) => id,
        }
    }
}
