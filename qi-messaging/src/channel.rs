use crate::{address::Address, binary_codec, Error, Message};
use async_stream::stream;
use futures::{Sink, SinkExt, Stream, TryStreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::{FramedRead, FramedWrite};

pub async fn connect<Body>(
    address: Address,
) -> Result<
    (
        impl Stream<Item = Result<Message<Body>, Error>>,
        impl Sink<Message<Body>, Error = Error>,
    ),
    std::io::Error,
>
where
    Body: crate::Body,
    Body::Error: std::error::Error + Sync + Send + 'static,
{
    let (read, write) = match address {
        Address::Tcp { address, ssl: None } => {
            let (read, write) = TcpStream::connect(address).await?.into_split();
            (Box::pin(read), Box::pin(write))
        }
        _ => unimplemented!(),
    };
    Ok(rw_to_messages_stream_sink(read, write))
}

pub async fn serve<Body>(
    address: Address,
) -> Result<
    (
        impl Stream<
            Item = (
                impl Stream<Item = Result<Message<Body>, Error>>,
                impl Sink<Message<Body>, Error = Error>,
                Address,
            ),
        >,
        Vec<Address>,
    ),
    std::io::Error,
>
where
    Body: crate::Body,
    Body::Error: std::error::Error + Sync + Send + 'static,
{
    match address {
        Address::Tcp { address, ssl } => {
            if ssl.is_some() {
                // TODO - handle listening as a SSL/TLS endpoint.
                unimplemented!("binding to a TCP endpoint with SSL is not yet supported")
            }
            let listener = TcpListener::bind(address).await?;
            let endpoints = listener
                .local_addr()
                .map(|address| Address::Tcp { address, ssl })
                .into_iter()
                .collect();
            let clients = stream! {
                loop {
                    // TODO: Handle case when accept returns an error that is fatal for this listener.
                    if let Ok((socket , address)) = listener.accept().await {
                        let (read, write) = socket.into_split();
                        let (stream, sink) = rw_to_messages_stream_sink(read, write);
                        yield (stream, sink, Address::Tcp { address, ssl });
                    }
                }
            };
            Ok((clients, endpoints))
        }
    }
}

fn rw_to_messages_stream_sink<R, W, Body>(
    read: R,
    write: W,
) -> (
    impl Stream<Item = Result<Message<Body>, Error>>,
    impl Sink<Message<Body>, Error = Error>,
)
where
    R: AsyncRead,
    W: AsyncWrite,
    Body: crate::Body,
    Body::Error: std::error::Error + Sync + Send + 'static,
{
    (
        FramedRead::new(read, binary_codec::Decoder::new()).err_into(),
        FramedWrite::new(write, binary_codec::Encoder).sink_err_into(),
    )
}
