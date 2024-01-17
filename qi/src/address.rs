use std::{net::IpAddr, str::FromStr};

use iri_string::types::{UriStr, UriString};

use crate::Error;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Address {
    Tcp {
        host: String,
        port: u16,
        ssl: Option<SslKind>,
    },
}

impl Address {
    pub fn from_uri(uri: &UriStr) -> Result<Self, Error> {
        match uri.scheme_str() {
            "tcp" => {
                let (host, port) = tcp_host_and_port(uri)?;
                Ok(Self::Tcp {
                    host,
                    port,
                    ssl: None,
                })
            }
            "tcps" => {
                let (host, port) = tcp_host_and_port(uri)?;
                Ok(Self::Tcp {
                    host,
                    port,
                    ssl: Some(SslKind::Simple),
                })
            }
            "tcpsm" => {
                let (host, port) = tcp_host_and_port(uri)?;
                Ok(Self::Tcp {
                    host,
                    port,
                    ssl: Some(SslKind::Mutual),
                })
            }
            scheme => Err(Error::UnsupportedUriScheme(scheme.to_owned())),
        }
    }

    pub fn is_machine_local(&self) -> bool {
        match self {
            Address::Tcp { host, .. } => IpAddr::from_str(host)
                .ok()
                .map_or(false, |addr| addr.is_loopback()),
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
            Address::Tcp { host, port, ssl } => {
                let protocol = match ssl {
                    None => "tcp",
                    Some(SslKind::Simple) => "tcps",
                    Some(SslKind::Mutual) => "tcpsm",
                };
                write!(f, "{protocol}://{host}:{port}")
            }
        }
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

fn tcp_host_and_port(uri: &UriStr) -> Result<(String, u16), Error> {
    const DEFAULT_HOST: &str = "localhost";
    const DEFAULT_PORT: u16 = 9559;
    let authority = uri.authority_components();
    Ok(match authority {
        Some(authority) => {
            let host = authority.host();
            let port = authority
                .port()
                .map(std::str::FromStr::from_str)
                .transpose()
                .map_err(Error::InvalidUriPort)?;
            (host.to_owned(), port.unwrap_or(DEFAULT_PORT))
        }
        None => (DEFAULT_HOST.to_owned(), DEFAULT_PORT),
    })
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SslKind {
    #[default]
    Simple,
    Mutual,
}
