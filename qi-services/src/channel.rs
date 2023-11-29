use crate::Error;
use iri_string::types::{UriStr, UriString};
use qi_messaging as messaging;
use qi_value as value;
use std::{net::IpAddr, str::FromStr};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

type MessagesIn = FramedRead<Box<dyn AsyncRead + Send + Unpin>, messaging::codec::Decoder>;
type MessagesOut = FramedWrite<Box<dyn AsyncWrite + Send + Unpin>, messaging::codec::Encoder>;

pub(crate) async fn open(address: Address) -> Result<(MessagesIn, MessagesOut), Error> {
    let (read, write) = match address {
        Address::Tcp {
            host,
            port,
            ssl: None,
        } => {
            let stream = TcpStream::connect((host, port)).await?;
            let (read, write) = stream.into_split();
            (Box::new(read), Box::new(write))
        }
        Address::Qi { .. } => return Err(Error::CannotOpenChannelOnRelativeAddress),
        _ => todo!(),
    };
    let incoming = MessagesIn::new(read, messaging::codec::Decoder::new());
    let outgoing = MessagesOut::new(write, messaging::codec::Encoder);
    Ok((incoming, outgoing))
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Address {
    Qi {
        service: String,
    },
    Tcp {
        host: String,
        port: u16,
        ssl: Option<SslKind>,
    },
}

impl Address {
    pub fn from_uri(uri: &UriStr) -> Result<Self, Error> {
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
        match uri.scheme_str() {
            "qi" => Ok(Self::Qi {
                service: uri.path_str().to_owned(),
            }),
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

    pub fn is_relative(&self) -> bool {
        matches!(self, Address::Qi { .. })
    }

    pub fn is_loopback(&self) -> bool {
        match self {
            Address::Tcp { host, .. } => IpAddr::from_str(host)
                .ok()
                .map_or(false, |addr| addr.is_loopback()),
            _ => false,
        }
    }

    pub fn as_relative(&self) -> Option<&String> {
        match self {
            Address::Qi { service } => Some(service),
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
            Address::Qi { service } => write!(f, "qi:{service}"),
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
