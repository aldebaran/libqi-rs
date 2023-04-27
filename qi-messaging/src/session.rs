use crate::{
    capabilities,
    channel::Channel,
    service::{client::Client, Request, Service},
};
use futures::{future::LocalBoxFuture, FutureExt};
use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct Session<S> {
    channel: Channel<Client<Error>, S>,
    capabilities: capabilities::Map,
}

impl Session {
    pub fn capabilities(&self) -> &capabilities::Map {
        &self.capabilities
    }
}

impl<C, S> tower::Service<Request> for Session<S>
where
    C: Service,
{
    type Response = C::Response;
    type Error = Error<C::Error>;
    type Future = C::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.channel.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.channel.call(req)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error<E>(#[from] E);

pub struct Run<'a, E> {
    inner: LocalBoxFuture<'a, Result<(), RunError<E>>>,
}

impl<'a, E> Debug for Run<'a, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Run").field("inner", &"dyn Future").finish()
    }
}

impl<'a, E> Future for Run<'a, E> {
    type Output = Result<(), RunError<E>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct RunError<E>(#[from] E);
