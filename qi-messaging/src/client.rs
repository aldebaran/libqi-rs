use crate::{
    endpoint::{self, ClientRequest},
    message,
};
use bytes::Bytes;
use futures::{ready, FutureExt, SinkExt};
use qi_format as format;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::oneshot;
use tokio_util::sync::{CancellationToken, DropGuard, PollSendError, PollSender};

#[derive(Clone)]
pub struct Client {
    requests: PollSender<endpoint::ClientRequest>,
}

impl Client {
    pub(crate) fn new(requests: PollSender<ClientRequest>) -> Self {
        Self { requests }
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Client")
    }
}

impl<T> tower::Service<Call<T>> for Client
where
    T: serde::Serialize + Send + 'static,
{
    type Response = Bytes;
    type Error = Error;
    type Future = Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_reserve(cx)?.map(Ok)
    }

    fn call(&mut self, call: Call<T>) -> Future {
        let (response_sender, response_receiver) = oneshot::channel();
        let cancel_token = CancellationToken::new();
        let request = ClientRequest::Call {
            address: call.address,
            args: Box::new(call.args),
            cancel_token: cancel_token.clone(),
            response_sender,
        };
        let inner = match self.requests.send_item(request) {
            Ok(()) => InnerFuture::RequestSent {
                response_receiver,
                drop_guard: Some(cancel_token.drop_guard()),
            },
            Err(_send_err) => InnerFuture::SendError,
        };
        Future(inner)
    }
}

impl<T> futures::Sink<Post<T>> for Client
where
    T: serde::Serialize + Send + 'static,
{
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_ready_unpin(cx)?.map(Ok)
    }

    fn start_send(mut self: Pin<&mut Self>, post: Post<T>) -> Result<(), Self::Error> {
        Ok(self.requests.start_send_unpin(ClientRequest::Post {
            address: post.address,
            value: Box::new(post.value),
        })?)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_flush_unpin(cx)?.map(Ok)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_close_unpin(cx)?.map(Ok)
    }
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Call<T> {
    pub(crate) address: message::Address,
    pub(crate) args: T,
}

impl<T> Call<T> {
    pub fn new(address: message::Address, args: T) -> Self {
        Self { address, args }
    }
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Post<T> {
    pub(crate) address: message::Address,
    pub(crate) value: T,
}

impl<T> Post<T> {
    pub fn new(address: message::Address, value: T) -> Self {
        Self { address, value }
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    derive_more::From,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum CallTermination {
    Error(String),
    Canceled,
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub struct Future(InnerFuture);

impl Future {
    pub fn cancel(&mut self) {
        if let InnerFuture::RequestSent { drop_guard, .. } = &mut self.0 {
            if let Some(guard) = drop_guard.take() {
                guard.disarm().cancel()
            }
        }
    }
}

impl std::future::Future for Future {
    type Output = Result<Bytes, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.0 {
            InnerFuture::SendError => Poll::Ready(Err(Error::Disconnected)),
            InnerFuture::RequestSent {
                response_receiver, ..
            } => {
                let reply = ready!(response_receiver.poll_unpin(cx)?)?;
                Poll::Ready(Ok(reply))
            }
        }
    }
}

#[derive(Debug)]
enum InnerFuture {
    SendError,
    RequestSent {
        response_receiver: oneshot::Receiver<Result<Bytes, Error>>,
        drop_guard: Option<DropGuard>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("client is disconnected")]
    Disconnected,

    #[error("format error")]
    Format(#[from] format::Error),

    #[error("canceled")]
    Canceled,

    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<PollSendError<ClientRequest>> for Error {
    fn from(_err: PollSendError<ClientRequest>) -> Self {
        Self::Disconnected
    }
}

impl From<oneshot::error::RecvError> for Error {
    fn from(_err: oneshot::error::RecvError) -> Self {
        Self::Disconnected
    }
}
