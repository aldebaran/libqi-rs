use crate::{node, Error};
use iri_string::types::{UriStr, UriString};
use qi_value as value;
use std::str::FromStr;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Address {
    /// An address relative to an existing service session.
    Relative {
        service: String,
    },
    Node(node::Address),
}

impl Address {
    pub fn from_uri(uri: &UriStr) -> Result<Self, Error> {
        match uri.scheme_str() {
            "qi" => Ok(Self::Relative {
                service: uri.path_str().to_owned(),
            }),
            _ => Ok(Self::Node(node::Address::from_uri(uri)?)),
        }
    }

    pub fn is_relative(&self) -> bool {
        matches!(self, Address::Relative { .. })
    }

    pub fn is_machine_local(&self) -> bool {
        match self {
            Self::Node(addr) => addr.is_machine_local(),
            _ => false,
        }
    }

    pub fn as_relative(&self) -> Option<&String> {
        match self {
            Address::Relative { service } => Some(service),
            _ => None,
        }
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = UriString::from_str(s)?;
        Self::from_uri(&uri)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Relative { service } => write!(f, "qi:{service}"),
            Self::Node(addr) => addr.fmt(f),
        }
    }
}

impl From<node::Address> for Address {
    fn from(addr: node::Address) -> Self {
        Self::Node(addr)
    }
}

impl TryFrom<&str> for Address {
    type Error = Error;
    fn try_from(str: &str) -> Result<Self, Self::Error> {
        Self::from_str(str)
    }
}

impl TryFrom<String> for Address {
    type Error = Error;
    fn try_from(str: String) -> Result<Self, Self::Error> {
        Self::from_str(&str)
    }
}

impl value::Reflect for Address {
    fn ty() -> Option<value::Type> {
        Some(value::Type::String)
    }
}

impl value::RuntimeReflect for Address {
    fn ty(&self) -> value::Type {
        value::Type::String
    }
}

impl<'a> value::FromValue<'a> for Address {
    fn from_value(value: value::Value<'a>) -> Result<Self, value::FromValueError> {
        let str: String = value.cast()?;
        str.parse()
            .map_err(|err: Error| value::FromValueError::Other(err.into()))
    }
}

impl<'a> value::IntoValue<'a> for Address {
    fn into_value(self) -> value::Value<'a> {
        value::Value::String(self.to_string().into())
    }
}

impl value::ToValue for Address {
    fn to_value(&self) -> value::Value<'_> {
        value::Value::String(self.to_string().into())
    }
}

impl<'a> TryFrom<value::Value<'a>> for Address {
    type Error = value::FromValueError;
    fn try_from(value: value::Value<'a>) -> Result<Self, Self::Error> {
        value::FromValue::from_value(value)
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SslKind {
    #[default]
    Simple,
    Mutual,
}

#[cfg(test)]
mod tests {
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
        let endpoints: Vec<Address> = input.deserialize_value().unwrap();
        assert_eq!(
            endpoints,
            [
                Address::Relative {
                    service: "Calculator".to_owned()
                },
                Address::Node(node::Address::Tcp {
                    host: "127.0.0.1".to_owned(),
                    port: 41681,
                    ssl: None
                })
            ]
        );
    }
}
