use crate::Error;
use std::{net::SocketAddr, str::FromStr};
use url::Url;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Address {
    Tcp {
        address: SocketAddr,
        ssl: Option<SslKind>,
    },
}

impl Address {
    pub fn from_url(url: &Url) -> Result<Self, Error> {
        match url.scheme().parse()? {
            Scheme::Tcp(ssl) => Ok(Self::Tcp {
                address: socket_addr_from_url(url)?,
                ssl,
            }),
        }
    }

    /// Returns true if the address refers to an endpoint of the local machine.
    ///
    /// # Resolution request and threading
    /// This will try to DNS resolve the host, using `std::net::ToSocketAddrs`,
    /// blocking the current thread until the request terminates. If the request
    /// fails, the address is considered not local.
    pub fn is_machine_local(&self) -> bool {
        match self {
            Address::Tcp { address, .. } => address.ip().is_loopback(),
        }
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_url(&s.parse()?)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp { address, ssl } => {
                let protocol = match ssl {
                    None => "tcp",
                    Some(SslKind::Simple) => "tcps",
                    Some(SslKind::Mutual) => "tcpsm",
                };
                write!(f, "{protocol}://{address}")
            }
        }
    }
}

fn socket_addr_from_url(url: &Url) -> Result<SocketAddr, Error> {
    const DEFAULT_PORT: u16 = 9559;
    url.socket_addrs(|| match Scheme::from_str(url.scheme()) {
        Ok(Scheme::Tcp(_)) => Some(DEFAULT_PORT),
        Err(_) => None,
    })?
    .first()
    .copied()
    .ok_or_else(|| Error::InvalidUrlHost(url.to_string()))
}

#[derive(Debug)]
enum Scheme {
    Tcp(Option<SslKind>),
}

impl FromStr for Scheme {
    type Err = Error;

    fn from_str(scheme: &str) -> Result<Self, Self::Err> {
        match scheme {
            "tcp" => Ok(Self::Tcp(None)),
            "tcps" => Ok(Self::Tcp(Some(SslKind::Simple))),
            "tcpsm" => Ok(Self::Tcp(Some(SslKind::Mutual))),
            _ => Err(Error::UnsupportedUrlScheme(scheme.to_owned())),
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SslKind {
    #[default]
    Simple,
    Mutual,
}
