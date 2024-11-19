use crate::{address::Address, binary_codec, /*Error,*/ Message};
use async_stream::stream;
use futures::{Sink, Stream, TryStream};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::{FramedRead, FramedWrite};

pub async fn connect<Body>(
    address: Address,
) -> Result<(impl TryStream<Ok = Message<Body>>, impl Sink<Message<Body>>), std::io::Error>
where
    Body: crate::Body,
{
    let (read, write) = match address {
        Address::Tcp { address, ssl: None } => {
            let (read, write) = TcpStream::connect(address).await?.into_split();
            (Box::pin(read), Box::pin(write))
        }
        _ => todo!(),
    };
    let stream = read_into_messages_stream(read);
    let sink = write_into_messages_sink(write);
    Ok((stream, sink))
}

pub async fn serve<Body>(
    address: Address,
) -> Result<
    (
        impl Stream<
            Item = (
                impl TryStream<Ok = Message<Body>>,
                impl Sink<Message<Body>>,
                Address,
            ),
        >,
        Vec<Address>,
    ),
    std::io::Error,
>
where
    Body: crate::Body,
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
                        let stream = read_into_messages_stream(read);
                        let sink = write_into_messages_sink(write);
                        yield (stream, sink, Address::Tcp { address, ssl });
                    }
                }
            };
            Ok((clients, endpoints))
        }
    }
}

fn read_into_messages_stream<Read, Body>(read: Read) -> impl TryStream<Ok = Message<Body>>
where
    Read: AsyncRead,
    Body: crate::Body,
{
    FramedRead::new(read, binary_codec::Decoder::new())
}

fn write_into_messages_sink<Write, Body>(write: Write) -> impl Sink<Message<Body>>
where
    Write: AsyncWrite,
    Body: crate::Body,
{
    FramedWrite::new(write, binary_codec::Encoder)
}
