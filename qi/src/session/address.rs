use crate::Error;
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    str::FromStr,
};
use url::Url;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Address {
    Tcp {
        address: SocketAddr,
        ssl: Option<SslKind>,
    },
}

impl Address {
    pub(crate) fn from_url(url: &Url) -> Result<Self, Error> {
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
    pub(crate) fn is_machine_local(&self) -> bool {
        match self {
            Address::Tcp { address, .. } => address.ip().is_loopback(),
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
    // We don't use `Url::socket_addrs` on purpose: the host must be an IP address and no resolution
    // is required. Another problem is that `socket_addrs` returns a list of addresses, which would
    // mean that we either have to select one of them arbitrarily, or we cannot parse a single
    // address from a string.
    const DEFAULT_HOST: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
    const DEFAULT_PORT: u16 = 9559;
    let host = match url.host() {
        Some(url::Host::Ipv4(addr)) => addr.into(),
        Some(url::Host::Ipv6(addr)) => addr.into(),
        Some(url::Host::Domain(domain)) => return Err(Error::InvalidUrlHost(domain.to_owned())),
        None => DEFAULT_HOST,
    };
    let port = url.port_or_known_default().unwrap_or(DEFAULT_PORT);
    Ok(SocketAddr::new(host, port))
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
