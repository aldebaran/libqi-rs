use crate::{Address, Error};
use qi_value as value;
use std::str::FromStr;
use url::Url;

/// A mean to refer to an existing session or otherwise to a session bound to be created.
///
/// It means that it potentially carries the information required to create a new session, such as
/// the address used to start a communication transport.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Reference(pub(crate) Inner);

impl Reference {
    #[cfg(test)]
    pub(crate) fn new_service(name: String) -> Self {
        Self(Inner::Service(name))
    }

    pub(crate) fn new_endpoint(address: Address) -> Self {
        Self(Inner::Endpoint(address))
    }

    pub fn from_url(url: &Url) -> Result<Self, Error> {
        match url.scheme() {
            "qi" => Ok(Self(Inner::Service(url.path().to_owned()))),
            _ => Ok(Self(Inner::Endpoint(Address::from_url(url)?))),
        }
    }

    pub(crate) fn is_service_relative(&self) -> bool {
        matches!(self.0, Inner::Service { .. })
    }

    pub(crate) fn as_service_relative(&self) -> Option<&String> {
        match &self.0 {
            Inner::Service(service) => Some(service),
            _ => None,
        }
    }

    pub(crate) fn into_endpoint(self) -> Option<Address> {
        match self.0 {
            Inner::Endpoint(address) => Some(address),
            _ => None,
        }
    }

    pub(crate) fn is_machine_local(&self) -> bool {
        match &self.0 {
            Inner::Endpoint(address) => address.is_machine_local(),
            _ => false,
        }
    }
}

impl FromStr for Reference {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_url(&s.parse()?)
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Inner::Service(service) => write!(f, "qi:{service}"),
            Inner::Endpoint(addr) => addr.fmt(f),
        }
    }
}

impl From<Address> for Reference {
    fn from(addr: Address) -> Self {
        Self(Inner::Endpoint(addr))
    }
}

impl TryFrom<&str> for Reference {
    type Error = Error;
    fn try_from(str: &str) -> Result<Self, Self::Error> {
        Self::from_str(str)
    }
}

impl TryFrom<String> for Reference {
    type Error = Error;
    fn try_from(str: String) -> Result<Self, Self::Error> {
        Self::from_str(&str)
    }
}

impl value::Reflect for Reference {
    fn ty() -> Option<value::Type> {
        Some(value::Type::String)
    }
}

impl value::RuntimeReflect for Reference {
    fn ty(&self) -> value::Type {
        value::Type::String
    }
}

impl<'a> value::FromValue<'a> for Reference {
    fn from_value(value: value::Value<'a>) -> Result<Self, value::FromValueError> {
        let str: String = value.cast_into()?;
        str.parse()
            .map_err(|err: Error| value::FromValueError::Other(err.into()))
    }
}

impl<'a> value::IntoValue<'a> for Reference {
    fn into_value(self) -> value::Value<'a> {
        value::Value::String(self.to_string().into())
    }
}

impl value::ToValue for Reference {
    fn to_value(&self) -> value::Value<'_> {
        value::Value::String(self.to_string().into())
    }
}

impl<'a> TryFrom<value::Value<'a>> for Reference {
    type Error = value::FromValueError;
    fn try_from(value: value::Value<'a>) -> Result<Self, Self::Error> {
        value::FromValue::from_value(value)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) enum Inner {
    /// A reference to an existing service session.
    Service(String),
    /// A reference to the address of an endpoint, that potentially requires opening a new channel
    /// to that endpoint and establishing the session over it.
    Endpoint(Address),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Address;
    use std::net::Ipv4Addr;

    #[test]
    fn test_addresses_deserialize() {
        let mut input = &[
            0x02, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x71, 0x69, 0x3a, 0x43, 0x61, 0x6c,
            0x63, 0x75, 0x6c, 0x61, 0x74, 0x6f, 0x72, 0x15, 0x00, 0x00, 0x00, 0x74, 0x63, 0x70,
            0x3a, 0x2f, 0x2f, 0x31, 0x32, 0x37, 0x2e, 0x30, 0x2e, 0x30, 0x2e, 0x31, 0x3a, 0x34,
            0x31, 0x36, 0x38, 0x31,
        ][..];
        let endpoints: Vec<Reference> = input.deserialize_value().unwrap();
        assert_eq!(
            endpoints,
            [
                Reference(Inner::Service("Calculator".to_owned())),
                Reference(Inner::Endpoint(Address::Tcp {
                    address: std::net::SocketAddrV4::new(Ipv4Addr::LOCALHOST, 41681).into(),
                    ssl: None
                }))
            ]
        );
    }
}
