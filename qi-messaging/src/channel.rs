use crate::codec::Codec;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

pub fn open_over_io<IO, Svc>(io: IO) -> Framed<IO, Codec>
where
    IO: AsyncWrite + AsyncRead,
{
    Framed::new(io, Codec::new())
}
