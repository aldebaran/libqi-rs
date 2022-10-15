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
pub enum Object {
    #[default]
    None,
    ServiceMain,
    Other(Id),
}

impl Object {
    const ID_NONE: u32 = 0;
    const ID_SERVICE_MAIN: u32 = 1;
}

impl From<Id> for Object {
    fn from(id: Id) -> Self {
        match id {
            Id(Self::ID_NONE) => Self::None,
            Id(Self::ID_SERVICE_MAIN) => Self::ServiceMain,
            id => Self::Other(id),
        }
    }
}

impl From<Object> for Id {
    fn from(o: Object) -> Self {
        match o {
            Object::None => Id(Object::ID_NONE),
            Object::ServiceMain => Id(Object::ID_SERVICE_MAIN),
            Object::Other(id) => id,
        }
    }
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id: Id = (*self).into();
        match self {
            Self::None => write!(f, "None({})", id),
            Self::ServiceMain => write!(f, "ServiceMain({})", id),
            Self::Other(_) => id.fmt(f),
        }
    }
}
