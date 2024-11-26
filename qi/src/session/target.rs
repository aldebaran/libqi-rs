use qi_messaging::{Address, AddressError};
use qi_value as value;
use serde_with::serde_as;
use std::str::FromStr;
use url::Url;

/// The session target either carries the information required to create a new session, such as
/// the address used to start a communication transport, or the information to retrieve an existing
/// suitable session.
#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde_with::SerializeDisplay,
    serde_with::DeserializeFromStr,
)]
#[serde_as]
pub struct Target(Kind);

impl Target {
    #[cfg(test)]
    pub(crate) fn service(name: impl ToString) -> Self {
        Self(Kind::Service(name.to_string()))
    }

    pub(crate) fn kind(&self) -> &Kind {
        &self.0
    }

    pub(super) fn from_url(url: &Url) -> Result<Self, AddressError> {
        match url.scheme() {
            "qi" => Ok(Self(Kind::Service(url.path().to_owned()))),
            _ => Ok(Self(Kind::Endpoint(Address::from_url(url)?))),
        }
    }

    pub(crate) fn is_service_relative(&self) -> bool {
        matches!(self.0, Kind::Service { .. })
    }

    pub(crate) fn is_machine_local(&self) -> bool {
        match &self.0 {
            Kind::Endpoint(address) => address.is_machine_local(),
            _ => false,
        }
    }
}

impl FromStr for Target {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_url(&s.parse()?).map_err(Into::into)
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Kind::Service(service) => write!(f, "qi:{service}"),
            Kind::Endpoint(addr) => addr.fmt(f),
        }
    }
}

impl From<Address> for Target {
    fn from(addr: Address) -> Self {
        Self(Kind::Endpoint(addr))
    }
}

impl TryFrom<&str> for Target {
    type Error = Error;
    fn try_from(str: &str) -> Result<Self, Self::Error> {
        Self::from_str(str)
    }
}

impl TryFrom<String> for Target {
    type Error = Error;
    fn try_from(str: String) -> Result<Self, Self::Error> {
        Self::from_str(&str)
    }
}

impl value::Reflect for Target {
    fn ty() -> Option<value::Type> {
        Some(value::Type::String)
    }
}

impl value::RuntimeReflect for Target {
    fn ty(&self) -> value::Type {
        value::Type::String
    }
}

impl<'a> value::FromValue<'a> for Target {
    fn from_value(value: value::Value<'a>) -> Result<Self, value::FromValueError> {
        let str: String = value.cast_into()?;
        str.parse()
            .map_err(|err: Error| value::FromValueError::Other(err.into()))
    }
}

impl<'a> value::IntoValue<'a> for Target {
    fn into_value(self) -> value::Value<'a> {
        value::Value::String(self.to_string().into())
    }
}

impl value::ToValue for Target {
    fn to_value(&self) -> value::Value<'_> {
        value::Value::String(self.to_string().into())
    }
}

impl<'a> TryFrom<value::Value<'a>> for Target {
    type Error = value::FromValueError;
    fn try_from(value: value::Value<'a>) -> Result<Self, Self::Error> {
        value::FromValue::from_value(value)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) enum Kind {
    /// The target is the name of an existing service session.
    Service(String),
    /// The target is the address of an endpoint, that potentially requires opening a new channel
    /// to that endpoint and establishing the session over it.
    Endpoint(Address),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),

    #[error(transparent)]
    Address(#[from] AddressError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::BinaryFormattedValue;
    use qi_messaging::Body;
    use std::net::Ipv4Addr;

    #[test]
    fn references_from_binary_value() {
        let binvalue = BinaryFormattedValue::from_static(&[
            0x02, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x71, 0x69, 0x3a, 0x43, 0x61, 0x6c,
            0x63, 0x75, 0x6c, 0x61, 0x74, 0x6f, 0x72, 0x15, 0x00, 0x00, 0x00, 0x74, 0x63, 0x70,
            0x3a, 0x2f, 0x2f, 0x31, 0x32, 0x37, 0x2e, 0x30, 0x2e, 0x30, 0x2e, 0x31, 0x3a, 0x34,
            0x31, 0x36, 0x38, 0x31,
        ]);
        let endpoints: Vec<Target> = binvalue.deserialize().unwrap();
        assert_eq!(
            endpoints,
            [
                Target(Kind::Service("Calculator".to_owned())),
                Target(Kind::Endpoint(Address::Tcp {
                    address: std::net::SocketAddrV4::new(Ipv4Addr::LOCALHOST, 41681).into(),
                    ssl: None
                }))
            ]
        );
    }
}
