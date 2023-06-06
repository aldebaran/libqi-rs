use super::{control, service};
use crate::{format, request::IsCanceled};
use bytes::Bytes;
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

impl<C, S> tower::Service<crate::request::Request> for Router<C, S>
where
    C: tower::Service<control::request::Request, Response = Option<control::capabilities::Map>>,
    S: tower::Service<service::Request>,
    S::Response: Into<Option<Bytes>>,
{
    type Response = Option<Bytes>;
    type Error = Error<C::Error, S::Error>;
    type Future = Future<C::Future, S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.control.poll_ready(cx)).map_err(Error::Control)?;
        ready!(self.service.poll_ready(cx)).map_err(Error::Service)?;
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: crate::request::Request) -> Self::Future {
        match control::request::Request::try_from_messaging(request) {
            Err(err) => Future::FormatError { error: Some(err) },
            Ok(Ok(request)) => {
                let call = self.control.call(request);
                Future::Control { inner: call }
            }
            Ok(Err(request)) => match service::Request::try_from_messaging(request) {
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
pub(super) enum Error<C, S> {
    #[error("control error")]
    Control(#[source] C),

    #[error("service error: {0}")]
    Service(#[source] S),

    #[error("format error")]
    Format(#[from] format::Error),

    #[error("the request could not be handled")]
    UnhandledRequest,
}

impl<C, S> IsCanceled for Error<C, S>
where
    C: IsCanceled,
    S: IsCanceled,
{
    fn is_canceled(&self) -> bool {
        match self {
            Self::Control(control) => control.is_canceled(),
            Self::Service(service) => service.is_canceled(),
            _ => false,
        }
    }
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
    C: TryFuture<Ok = Option<control::capabilities::Map>>,
    S: TryFuture,
    S::Ok: Into<Option<Bytes>>,
{
    type Output = Result<Option<Bytes>, Error<C::Error, S::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            FutureProj::Control { inner } => {
                let response = match ready!(inner.try_poll(cx)).map_err(Error::Control)? {
                    Some(control_response) => {
                        Some(format::to_bytes(&control_response).map_err(Error::Format)?)
                    }
                    None => None,
                };
                Poll::Ready(Ok(response))
            }
            FutureProj::Service { inner } => {
                let response = ready!(inner.try_poll(cx)).map_err(Error::Service)?;
                Poll::Ready(Ok(response.into()))
            }
            FutureProj::FormatError { error } => match error.take() {
                Some(err) => Poll::Ready(Err(Error::Format(err))),
                None => Poll::Pending,
            },
            FutureProj::UnhandledRequest => Poll::Ready(Err(Error::UnhandledRequest)),
        }
    }
}
