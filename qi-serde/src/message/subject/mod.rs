pub mod service;
pub use service::Service;

pub mod object;
pub use object::Object;

pub mod action;
pub use action::Action;

trait SubjectExt {
    fn service(&self) -> Service;
    fn object(&self) -> Object;
    fn action(&self) -> Action;
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Subject {
    Server(Server),
    ServiceDirectory(ServiceDirectory),
    BoundObject(BoundObject),
}

impl Subject {
    // Interpretation of action ID depends on service & object.
    pub fn try_from_values<S, O, A>(service: S, object: O, action_id: A) -> Result<Self, Error>
    where
        S: Into<Service>,
        O: Into<Object>,
        A: Into<action::Id>,
    {
        let service = service.into();
        let object = object.into();
        match (service, object) {
            (Service::Server, Object::None) => {
                let action = action_id.into().try_into()?;
                Ok(Server { action }.into())
            }
            (Service::Server, _) => Err(Error::UnexpectedServerObject(object)),
            (_, Object::None) => Err(Error::UnexpectedNoneObject),
            (Service::ServiceDirectory, Object::ServiceMain) => {
                let action = action_id.into().try_into()?;
                Ok(ServiceDirectory { action }.into())
            }
            (service, object) => {
                Ok(BoundObject::from_values_unchecked(service, object, action_id.into()).into())
            }
        }
    }
}

impl Default for Subject {
    fn default() -> Self {
        Self::Server(Server::default())
    }
}

impl SubjectExt for Subject {
    fn service(&self) -> Service {
        match self {
            Self::Server(s) => s.service(),
            Self::ServiceDirectory(sd) => sd.service(),
            Self::BoundObject(b) => b.service(),
        }
    }

    fn object(&self) -> Object {
        match self {
            Self::Server(s) => s.object(),
            Self::ServiceDirectory(sd) => sd.object(),
            Self::BoundObject(b) => b.object(),
        }
    }

    fn action(&self) -> Action {
        match self {
            Self::Server(s) => s.action(),
            Self::ServiceDirectory(sd) => sd.action(),
            Self::BoundObject(b) => b.action(),
        }
    }
}

impl From<Server> for Subject {
    fn from(s: Server) -> Self {
        Self::Server(s)
    }
}

impl From<ServiceDirectory> for Subject {
    fn from(sd: ServiceDirectory) -> Self {
        Self::ServiceDirectory(sd)
    }
}

impl From<BoundObject> for Subject {
    fn from(b: BoundObject) -> Self {
        Self::BoundObject(b)
    }
}

mod ser {
    use super::*;

    #[doc(hidden)]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename = "Subject")]
    pub(crate) struct Repr {
        pub(crate) service: Service,
        pub(crate) object: Object,
        pub(crate) action: action::Id,
    }

    impl serde::Serialize for Subject {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Repr {
                service: self.service(),
                object: self.object(),
                action: self.action().into(),
            }
            .serialize(serializer)
        }
    }
}

mod de {
    use super::*;

    impl<'de> serde::Deserialize<'de> for Subject {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = ser::Repr::deserialize(deserializer)?;
            Self::try_from_values(s.service, s.object, s.action).map_err(serde::de::Error::custom)
        }
    }
}

#[derive(thiserror::Error, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    #[error("server object ({0}) is not none ({})", Object::None)]
    UnexpectedServerObject(Object),

    #[error("unexpected \"none\" object")]
    UnexpectedNoneObject,

    #[error("{0}")]
    BadServerAction(#[from] action::ServerError),

    #[error("{0}")]
    BadServiceDirectoryAction(#[from] action::ServiceDirectoryError),
}

// service = server, object = none
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
#[serde(try_from = "Subject")]
#[serde(into = "Subject")]
pub struct Server {
    pub action: action::Server,
}

impl SubjectExt for Server {
    fn service(&self) -> Service {
        Service::Server
    }

    fn object(&self) -> Object {
        Object::None
    }

    fn action(&self) -> Action {
        self.action.into()
    }
}

impl TryFrom<Subject> for Server {
    type Error = TryFromSubjectError;

    fn try_from(value: Subject) -> Result<Self, Self::Error> {
        match value {
            Subject::Server(s) => Ok(s),
            _ => Err(TryFromSubjectError),
        }
    }
}

// service = service directory, object = main
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
#[serde(try_from = "Subject")]
#[serde(into = "Subject")]
pub struct ServiceDirectory {
    pub action: action::ServiceDirectory,
}

impl SubjectExt for ServiceDirectory {
    fn service(&self) -> Service {
        Service::ServiceDirectory
    }

    fn object(&self) -> Object {
        Object::ServiceMain
    }

    fn action(&self) -> Action {
        self.action.into()
    }
}

impl TryFrom<Subject> for ServiceDirectory {
    type Error = TryFromSubjectError;

    fn try_from(value: Subject) -> Result<Self, Self::Error> {
        match value {
            Subject::ServiceDirectory(s) => Ok(s),
            _ => Err(TryFromSubjectError),
        }
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
#[serde(try_from = "Subject")]
#[serde(into = "Subject")]
pub struct BoundObject {
    service: Service,
    object: Object,
    action: action::BoundObject,
}

impl BoundObject {
    pub(crate) fn from_values_unchecked<S, O, A>(service: S, object: O, action: A) -> Self
    where
        S: Into<Service>,
        O: Into<Object>,
        A: Into<action::BoundObject>,
    {
        let (service, object, action) = (service.into(), object.into(), action.into());
        debug_assert!(
            object != Object::None
                && !(service == Service::ServiceDirectory && object == Object::ServiceMain),
            "bad BoundObject subject values {:?} {:?}",
            service,
            object,
        );
        Self {
            service,
            object,
            action,
        }
    }
}

impl SubjectExt for BoundObject {
    fn service(&self) -> Service {
        self.service
    }

    fn object(&self) -> Object {
        self.object
    }

    fn action(&self) -> Action {
        self.action.into()
    }
}

impl TryFrom<Subject> for BoundObject {
    type Error = TryFromSubjectError;

    fn try_from(value: Subject) -> Result<Self, Self::Error> {
        match value {
            Subject::BoundObject(b) => Ok(b),
            _ => Err(TryFromSubjectError),
        }
    }
}

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct TryFromSubjectError;

impl std::fmt::Display for TryFromSubjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("conversion error between subject types")
    }
}

impl std::error::Error for TryFromSubjectError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_try_from_values_server() {
        assert_eq!(
            Subject::try_from_values(service::Id(0), object::Id(0), action::Id(8)),
            Ok(Subject::from(Server {
                action: action::Server::Authenticate,
            }))
        );
    }

    #[test]
    fn test_try_from_values_server_unexpected_object() {
        assert_eq!(
            Subject::try_from_values(service::Id(0), object::Id(1), action::Id(190)),
            Err(Error::UnexpectedServerObject(Object::ServiceMain))
        );
    }

    #[test]
    fn test_try_from_values_server_bad_action() {
        assert_eq!(
            Subject::try_from_values(service::Id(0), object::Id(0), action::Id(120)),
            Err(Error::BadServerAction(action::ServerError(action::Id(120))))
        );
    }

    #[test]
    fn test_try_from_values_service_directory() {
        assert_eq!(
            Subject::try_from_values(service::Id(1), object::Id(1), action::Id(103)),
            Ok(Subject::from(ServiceDirectory {
                action: action::ServiceDirectory::UnregisterService
            }))
        );
    }

    #[test]
    fn test_try_from_values_service_directory_unexpected_none_object() {
        assert_eq!(
            Subject::try_from_values(service::Id(1), object::Id(0), action::Id(106)),
            Err(Error::UnexpectedNoneObject)
        );
    }

    #[test]
    fn test_try_from_values_service_directory_bad_action() {
        assert_eq!(
            Subject::try_from_values(service::Id(1), object::Id(1), action::Id(932)),
            Err(Error::BadServiceDirectoryAction(
                action::ServiceDirectoryError(action::Id(932))
            ))
        );
    }

    #[test]
    fn test_try_from_values_bound_object_bound_function() {
        assert_eq!(
            Subject::try_from_values(service::Id(39), object::Id(903), action::Id(329)),
            Ok(Subject::from(BoundObject {
                service: Service::Other(service::Id(39)),
                object: Object::Other(object::Id(903)),
                action: action::BoundObject::BoundFunction(action::Id(329)),
            }))
        );
    }

    #[test]
    fn test_try_from_values_bound_object_special_function() {
        assert_eq!(
            Subject::try_from_values(service::Id(1093), object::Id(89271), action::Id(6)),
            Ok(Subject::from(BoundObject {
                service: Service::Other(service::Id(1093)),
                object: Object::Other(object::Id(89271)),
                action: action::BoundObject::SetProperty,
            }))
        );
    }

    #[test]
    fn test_try_from_values_bound_object_unexpected_none_object() {
        assert_eq!(
            Subject::try_from_values(service::Id(329), object::Id(0), action::Id(921)),
            Err(Error::UnexpectedNoneObject)
        );
    }

    #[test]
    fn test_ser_de_server() {
        assert_tokens(
            &Subject::Server(Server {
                action: action::Server::Authenticate,
            }),
            &[
                Token::Struct {
                    name: "Subject",
                    len: 3,
                },
                Token::Str("service"),
                Token::U32(0),
                Token::Str("object"),
                Token::U32(0),
                Token::Str("action"),
                Token::U32(8),
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn test_ser_de_service_directory() {
        assert_tokens(
            &Subject::ServiceDirectory(ServiceDirectory {
                action: action::ServiceDirectory::UpdateServiceInfo,
            }),
            &[
                Token::Struct {
                    name: "Subject",
                    len: 3,
                },
                Token::Str("service"),
                Token::U32(1),
                Token::Str("object"),
                Token::U32(1),
                Token::Str("action"),
                Token::U32(105),
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn test_ser_de_bound_object() {
        assert_tokens(
            &Subject::BoundObject(BoundObject {
                service: Service::Other(service::Id(1093)),
                object: Object::Other(object::Id(89271)),
                action: action::BoundObject::SetProperty,
            }),
            &[
                Token::Struct {
                    name: "Subject",
                    len: 3,
                },
                Token::Str("service"),
                Token::U32(1093),
                Token::Str("object"),
                Token::U32(89271),
                Token::Str("action"),
                Token::U32(6),
                Token::StructEnd,
            ],
        );
    }
}
