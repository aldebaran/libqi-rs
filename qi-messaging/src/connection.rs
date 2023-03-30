use crate::{
    codec,
    dispatch::{self, Dispatch},
};
use futures::{ready, FutureExt};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{split, AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio_util::codec::{FramedRead, FramedWrite};

#[derive(Debug)]
pub struct Connection<IO> {
    dispatch: Dispatch<
        FramedWrite<WriteHalf<IO>, codec::Encoder>,
        FramedRead<ReadHalf<IO>, codec::Decoder>,
    >,
}

impl<IO> Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    pub(crate) fn new(io: IO) -> (Self, dispatch::OrderSender) {
        let (input, output) = split(io);
        let sink = FramedWrite::new(output, codec::Encoder);
        let stream = FramedRead::new(input, codec::Decoder::new());
        let (dispatch, orders) = Dispatch::new(sink, stream);
        (Self { dispatch }, orders)
    }
}

impl<IO> Future for Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let term = ready!(self.dispatch.poll_unpin(cx));
        let res = match term {
            dispatch::Termination::ClientDropped => Ok(()),
            dispatch::Termination::InputClosed => Ok(()),
            dispatch::Termination::InputError(err) => Err(Error::IO(err)),
            dispatch::Termination::OutputError(err) => Err(Error::IO(err)),
        };
        Poll::Ready(res)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error")]
    IO(#[from] std::io::Error),
}
