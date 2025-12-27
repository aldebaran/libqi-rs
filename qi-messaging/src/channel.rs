use crate::{address::Address, DecodeError, Decoder, EncodeError, Encoder, Message};
use async_stream::stream;
use futures::{Sink, Stream};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite};

pub async fn connect(
    address: Address,
) -> Result<
    (
        impl Stream<Item = Result<Message, DecodeError>>,
        impl Sink<Message, Error = EncodeError>,
    ),
    std::io::Error,
> {
    let (read, write) = match address {
        Address::Tcp { address, ssl: None } => {
            let (read, write) = TcpStream::connect(address).await?.into_split();
            (Box::pin(read), Box::pin(write))
        }
        _ => todo!(),
    };
    let stream = FramedRead::new(read, Decoder::default());
    let sink = FramedWrite::new(write, Encoder::default());
    Ok((stream, sink))
}

pub async fn serve(
    address: Address,
) -> Result<
    (
        impl Stream<
            Item = (
                impl Stream<Item = Result<Message, DecodeError>>,
                impl Sink<Message, Error = EncodeError>,
                Address,
            ),
        >,
        Address,
    ),
    std::io::Error,
> {
    match address {
        Address::Tcp { address, ssl } => {
            if ssl.is_some() {
                // TODO - handle listening as a SSL/TLS endpoint.
                unimplemented!("binding to a TCP endpoint with SSL is not yet supported")
            }
            let listener = TcpListener::bind(address).await?;
            let endpoint = listener
                .local_addr()
                .map(|address| Address::Tcp { address, ssl })
                .unwrap_or_else(|_err| Address::Tcp { address, ssl });
            let clients = stream! {
                loop {
                    // TODO: Handle case when accept returns an error that is fatal for this listener.
                    if let Ok((socket , address)) = listener.accept().await {
                        let (read, write) = socket.into_split();
                        let stream = FramedRead::new(read, Decoder::default());
                        let sink = FramedWrite::new(write, Encoder::default());
                        yield (stream, sink, Address::Tcp { address, ssl });
                    }
                }
            };
            Ok((clients, endpoint))
        }
    }
}
