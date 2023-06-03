use super::{control, service};
use crate::{
    format,
    request::{Request as MessagingRequest, Response as MessagingResponse},
};
use futures::{ready, TryFuture};
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(derive_new::new, Debug)]
pub(super) struct Router<C, S> {
    control: C,
    service: S,
}

impl<C, S> tower::Service<MessagingRequest> for Router<C, S>
where
    C: tower::Service<control::Request, Response = control::Response>,
    S: tower::Service<service::Request, Response = service::Response>,
{
    type Response = MessagingResponse;
    type Error = Error<C::Error, S::Error>;
    type Future = Future<C::Future, S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.control.poll_ready(cx)).map_err(Error::Control)?;
        ready!(self.service.poll_ready(cx)).map_err(Error::Service)?;
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: MessagingRequest) -> Self::Future {
        match control::Request::try_from_messaging_request(request) {
            Err(err) => Future::FormatError { error: Some(err) },
            Ok(Ok(request)) => {
                let call = self.control.call(request);
                Future::Control { inner: call }
            }
            Ok(Err(request)) => match service::Request::try_from_messaging_request(request) {
                Ok(request) => {
                    let call = self.service.call(request);
                    Future::Service { inner: call }
                }
                Err(_) => Future::UnhandledRequest,
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub(super) enum Error<C, S> {
    #[error("control service error")]
    Control(C),

    #[error(transparent)]
    Service(S),

    #[error("format error")]
    FormatError(#[source] format::Error),

    #[error("the request could not be handled")]
    UnhandledRequest,
}

pin_project! {
    #[derive(Debug)]
    #[project = FutureProj]
    #[must_use = "futures do nothing until polled"]
    pub(super) enum Future<C, S> {
        Control {
            #[pin]
            inner: C
        },
        Service {
            #[pin]
            inner: S
        },
        FormatError {
            error: Option<format::Error>
        },
        UnhandledRequest,
    }
}

impl<C, S> std::future::Future for Future<C, S>
where
    C: TryFuture<Ok = control::Response>,
    S: TryFuture<Ok = service::Response>,
{
    type Output = Result<MessagingResponse, Error<C::Error, S::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            FutureProj::Control { inner } => {
                let response = ready!(inner.try_poll(cx)).map_err(Error::Control)?;
                let response = response
                    .try_into_messaging_response()
                    .map_err(Error::FormatError)?;
                Poll::Ready(Ok(response))
            }
            FutureProj::Service { inner } => {
                let response = ready!(inner.try_poll(cx)).map_err(Error::Service)?;
                Poll::Ready(Ok(response))
            }
            FutureProj::FormatError { error } => match error.take() {
                Some(err) => Poll::Ready(Err(Error::FormatError(err))),
                None => Poll::Pending,
            },
            FutureProj::UnhandledRequest => Poll::Ready(Err(Error::UnhandledRequest)),
        }
    }
}
