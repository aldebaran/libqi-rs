use crate::{Address, Error};
use iri_string::types::{UriStr, UriString};
use qi_value as value;
use std::str::FromStr;

/// A session reference is a mean to identify and/or reuse sessions to services or endpoints.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Reference {
    /// A reference to an existing service session.
    Service(String),
    /// A reference to the address of an endpoint, that potentially requires opening a new channel
    /// to that endpoint and establishing the session over it.
    Endpoint(Address),
}

impl Reference {
    pub fn from_uri(uri: &UriStr) -> Result<Self, Error> {
        match uri.scheme_str() {
            "qi" => Ok(Self::Service(uri.path_str().to_owned())),
            _ => Ok(Self::Endpoint(Address::from_uri(uri)?)),
        }
    }

    pub fn is_service_relative(&self) -> bool {
        matches!(self, Reference::Service { .. })
    }

    pub fn as_service_relative(&self) -> Option<&String> {
        match self {
            Self::Service(service) => Some(service),
            _ => None,
        }
    }

    pub fn is_machine_local(&self) -> bool {
        match self {
            Self::Endpoint(addr) => addr.is_machine_local(),
            _ => false,
        }
    }
}

impl FromStr for Reference {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = UriString::from_str(s)?;
        Self::from_uri(&uri)
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Service(service) => write!(f, "qi:{service}"),
            Self::Endpoint(addr) => addr.fmt(f),
        }
    }
}

impl From<Address> for Reference {
    fn from(addr: Address) -> Self {
        Self::Endpoint(addr)
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
        let str: String = value.cast()?;
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

#[cfg(test)]
mod tests {
    use crate::Address;

    use super::*;
    use qi_format::de::BufExt;

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
                Reference::Service("Calculator".to_owned()),
                Reference::Endpoint(Address::Tcp {
                    host: "127.0.0.1".to_owned(),
                    port: 41681,
                    ssl: None
                })
            ]
        );
    }
}
