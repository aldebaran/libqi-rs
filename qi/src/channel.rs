use crate::{Address, Error};
use qi_messaging as messaging;
use std::pin::Pin;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

type MessagesDecode = FramedRead<Pin<Box<dyn AsyncRead + Send>>, messaging::codec::Decoder>;
type MessagesEncode = FramedWrite<Pin<Box<dyn AsyncWrite + Send>>, messaging::codec::Encoder>;

fn decode_encode_messages<R, W>(read: R, write: W) -> (MessagesDecode, MessagesEncode)
where
    R: AsyncRead + Send + 'static,
    W: AsyncWrite + Send + 'static,
{
    let incoming = MessagesDecode::new(Box::pin(read), messaging::codec::Decoder::new());
    let outgoing = MessagesEncode::new(Box::pin(write), messaging::codec::Encoder);
    (incoming, outgoing)
}

pub(crate) async fn open(address: Address) -> Result<(MessagesDecode, MessagesEncode), Error> {
    match address {
        Address::Tcp {
            host,
            port,
            ssl: None,
        } => {
            let (read, write) = TcpStream::connect((host, port)).await?.into_split();
            Ok(decode_encode_messages(read, write))
        }
        _ => unimplemented!(),
    }
}
