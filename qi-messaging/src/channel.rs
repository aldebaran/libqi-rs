use crate::{address::Address, binary_codec, BodyBuf, Error, Message};
use async_stream::stream;
use futures::{Sink, SinkExt, Stream, TryStreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::{FramedRead, FramedWrite};

pub async fn connect<ReadBody, WriteBody>(
    address: Address,
) -> Result<
    (
        impl Stream<Item = Result<Message<ReadBody>, Error>>,
        impl Sink<Message<WriteBody>, Error = Error>,
    ),
    Error,
>
where
    ReadBody: BodyBuf,
    ReadBody::Error: std::error::Error + Sync + Send + 'static,
    for<'de> <ReadBody::Deserializer<'de> as serde::Deserializer<'de>>::Error:
        Into<ReadBody::Error>,
    WriteBody: BodyBuf,
    WriteBody::Error: std::error::Error + Send + Sync + 'static,
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

pub async fn serve<ReadBody, WriteBody>(
    address: Address,
) -> Result<
    (
        impl Stream<
            Item = (
                impl Stream<Item = Result<Message<ReadBody>, Error>>,
                impl Sink<Message<WriteBody>, Error = Error>,
                Address,
            ),
        >,
        Vec<Address>,
    ),
    Error,
>
where
    ReadBody: BodyBuf,
    ReadBody::Error: std::error::Error + Sync + Send + 'static,
    for<'de> <ReadBody::Deserializer<'de> as serde::Deserializer<'de>>::Error:
        Into<ReadBody::Error>,
    WriteBody: BodyBuf,
    WriteBody::Error: std::error::Error + Send + Sync + 'static,
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

fn rw_to_messages_stream_sink<R, W, ReadBody, WriteBody>(
    read: R,
    write: W,
) -> (
    impl Stream<Item = Result<Message<ReadBody>, Error>>,
    impl Sink<Message<WriteBody>, Error = Error>,
)
where
    R: AsyncRead,
    W: AsyncWrite,
    ReadBody: BodyBuf,
    ReadBody::Error: std::error::Error + Sync + Send + 'static,
    for<'de> <ReadBody::Deserializer<'de> as serde::Deserializer<'de>>::Error:
        Into<ReadBody::Error>,
    WriteBody: BodyBuf,
    WriteBody::Error: std::error::Error + Send + Sync + 'static,
{
    (
        FramedRead::new(read, binary_codec::Decoder::new()).err_into(),
        FramedWrite::new(write, binary_codec::Encoder).sink_err_into(),
    )
}
