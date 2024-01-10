use qi_messaging as messaging;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{FramedRead, FramedWrite};

type MessagesDecode<R> = FramedRead<R, messaging::codec::Decoder>;
type MessagesEncode<W> = FramedWrite<W, messaging::codec::Encoder>;

pub(crate) fn open_on_rw<R, W>(read: R, write: W) -> (MessagesDecode<R>, MessagesEncode<W>)
where
    R: AsyncRead,
    W: AsyncWrite,
{
    let incoming = MessagesDecode::new(read, messaging::codec::Decoder::new());
    let outgoing = MessagesEncode::new(write, messaging::codec::Encoder);
    (incoming, outgoing)
}
