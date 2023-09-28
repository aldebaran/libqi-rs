use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::Uri;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

const DEFAULT_TCP_PORT: u16 = 9559;

#[derive(Debug)]
pub(crate) enum Transport {
    Tcp(TcpStream),
}

impl Transport {
    pub(crate) async fn connect(uri: Uri) -> Result<Self, ConnectFromUriError> {
        match uri.scheme_str() {
            "tcp" => {
                let authority_components = uri
                    .authority_components()
                    .ok_or_else(|| ConnectFromUriError::MissingUriAuthority(uri.clone()))?;
                let port = match authority_components.port() {
                    Some(port) => {
                        port.parse()
                            .map_err(|source| ConnectFromUriError::ParseTcpPort {
                                uri: uri.clone(),
                                source,
                            })?
                    }
                    None => DEFAULT_TCP_PORT,
                };
                let address = (authority_components.host(), port);
                Ok(Self::Tcp(TcpStream::connect(address).await?))
            }
            scheme => Err(ConnectFromUriError::UnrecognizedUriScheme(
                scheme.to_owned(),
            )),
        }
    }
}

impl AsyncWrite for Transport {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            Transport::Tcp(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            Transport::Tcp(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            Transport::Tcp(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            Transport::Tcp(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            Transport::Tcp(stream) => stream.is_write_vectored(),
        }
    }
}

impl AsyncRead for Transport {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Transport::Tcp(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectFromUriError {
    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error("missing URI authority in \"{0}\"")]
    MissingUriAuthority(Uri),

    #[error("failed to parse a TCP port from URI \"{uri}\"")]
    ParseTcpPort {
        uri: Uri,
        source: std::num::ParseIntError,
    },

    #[error("unrecognized URI scheme \"{0}\"")]
    UnrecognizedUriScheme(String),
}
