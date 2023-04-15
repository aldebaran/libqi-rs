use crate::{codec, dispatch::Dispatch, message::Message};
use futures::{ready, Sink, SinkExt, StreamExt};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{split, AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio_util::codec::{FramedRead, FramedWrite};

#[derive(Debug)]
pub struct Connection<IO> {
    input: FramedRead<ReadHalf<IO>, codec::Decoder>,
    output: FramedWrite<WriteHalf<IO>, codec::Encoder>,
    dispatch: Dispatch,
    buffered_message_to_dispatch: Option<Message>,
    buffered_message_to_output: Option<Message>,
}

impl<IO> Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    pub(crate) fn new(io: IO, dispatch: Dispatch) -> Self {
        let (input, output) = split(io);
        let input = FramedRead::new(input, codec::Decoder::new());
        let output = FramedWrite::new(output, codec::Encoder);
        Self {
            input,
            output,
            dispatch,
            buffered_message_to_dispatch: None,
            buffered_message_to_output: None,
        }
    }

    fn poll_copy_input_to_dispatch(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        if let Some(msg) = self.buffered_message_to_dispatch.take() {
            if let Err(err) = ready!(self.poll_send_message_to_dispatch(msg, cx)) {
                match err {}
            }
        }

        loop {
            match self.input.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(msg))) => {
                    if let Err(err) = ready!(self.poll_send_message_to_dispatch(msg, cx)) {
                        match err {}
                    }
                }
                Poll::Ready(Some(Err(err))) => return Poll::Ready(Err(Error::IO(err))),
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => break,
            }
        }

        if let Err(err) = ready!(self.dispatch.poll_flush_unpin(cx)) {
            match err {}
        }

        Poll::Pending
    }

    fn poll_copy_dispatch_to_output(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        if let Some(msg) = self.buffered_message_to_output.take() {
            ready!(self.poll_send_message_to_output(msg, cx))?;
        }

        loop {
            match self.dispatch.poll_next_unpin(cx) {
                Poll::Ready(Some(msg)) => {
                    ready!(self.poll_send_message_to_output(msg, cx))?;
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => break,
            }
        }

        ready!(self.output.poll_flush_unpin(cx))?;

        Poll::Pending
    }

    fn poll_send_message_to_dispatch(
        &mut self,
        msg: Message,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), <Dispatch as Sink<Message>>::Error>> {
        debug_assert!(self.buffered_message_to_dispatch.is_none());
        match self.dispatch.poll_ready_unpin(cx) {
            Poll::Pending => {
                self.buffered_message_to_dispatch = Some(msg);
                return Poll::Pending;
            }
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
            Poll::Ready(Ok(())) => {}
        }
        self.dispatch.start_send_unpin(msg)?;
        Poll::Ready(Ok(()))
    }

    fn poll_send_message_to_output(
        &mut self,
        msg: Message,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        debug_assert!(self.buffered_message_to_output.is_none());
        match self.output.poll_ready_unpin(cx) {
            Poll::Pending => {
                self.buffered_message_to_output = Some(msg);
                return Poll::Pending;
            }
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
            Poll::Ready(Ok(())) => {}
        }
        self.output.start_send_unpin(msg)?;
        Poll::Ready(Ok(()))
    }
}

impl<IO> Future for Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(res) = self.poll_copy_input_to_dispatch(cx) {
            return Poll::Ready(res);
        }
        if let Poll::Ready(res) = self.poll_copy_dispatch_to_output(cx) {
            return Poll::Ready(res);
        }
        Poll::Pending
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error")]
    IO(#[from] std::io::Error),
}
