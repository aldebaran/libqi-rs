use super::{control, service};
use crate::{format, request::IsCanceledError};
use bytes::Bytes;
use futures::{ready, TryFuture};
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

#[derive(Debug)]
pub(super) struct Router<C, S> {
    control: C,
    service: Option<S>,
    enable_service_receiver: Option<oneshot::Receiver<EnableService<S>>>,
}

impl<C, S> Router<C, S>
where
    C: tower::Service<control::Request, Response = Option<control::capabilities::CapabilitiesMap>>,
    S: tower::Service<service::Request>,
    S::Response: Into<Option<Bytes>>,
{
    pub(super) fn new(control: C) -> (Self, oneshot::Sender<EnableService<S>>) {
        let (enable_service_sender, enable_service_receiver) = oneshot::channel();
        (
            Self {
                control,
                service: None,
                enable_service_receiver: Some(enable_service_receiver),
            },
            enable_service_sender,
        )
    }

    pub(super) fn new_service_enabled(control: C, service: S) -> Self {
        Self {
            control,
            service: Some(service),
            enable_service_receiver: None,
        }
    }

    fn enable_service(&mut self, service: S) {
        self.service.replace(service);
    }

    fn route_request(&mut self, request: crate::Request) -> Future<C::Future, S::Future> {
        match Request::from_messaging(request) {
            Err(error) => Future::FormatError { error: Some(error) },
            Ok(Ok(Request::Control(request))) => {
                Self::route_control_request(&mut self.control, request)
            }
            Ok(Ok(Request::Service(request))) => match self.service.as_mut() {
                Some(service) => Self::route_service_request(service, request),
                None => Future::UnhandledRequest,
            },
            _ => Future::UnhandledRequest,
        }
    }

    fn route_control_request(
        control: &mut C,
        request: control::Request,
    ) -> Future<C::Future, S::Future> {
        let inner = control.call(request);
        Future::Control { inner }
    }

    fn route_service_request(
        service: &mut S,
        request: service::Request,
    ) -> Future<C::Future, S::Future> {
        let inner = service.call(request);
        Future::Service { inner }
    }

    #[allow(clippy::type_complexity)]
    fn poll_command_and_services_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error<C::Error, S::Error>>> {
        if let Some(command) = self.enable_service_receiver.as_mut() {
            match command.try_recv() {
                Ok(EnableService(service)) => {
                    self.enable_service(service);
                    self.enable_service_receiver = None
                }
                Err(oneshot::error::TryRecvError::Closed) => self.enable_service_receiver = None,
                Err(oneshot::error::TryRecvError::Empty) => (),
            }
        }

        ready!(self.control.poll_ready(cx)).map_err(Error::Control)?;
        if let Some(service) = self.service.as_mut() {
            ready!(service.poll_ready(cx)).map_err(Error::Service)?;
        }
        Poll::Ready(Ok(()))
    }
}

impl<C, S> tower::Service<crate::Request> for Router<C, S>
where
    C: tower::Service<control::Request, Response = Option<control::capabilities::CapabilitiesMap>>,
    S: tower::Service<service::Request>,
    S::Response: Into<Option<Bytes>>,
{
    type Response = Option<Bytes>;
    type Error = Error<C::Error, S::Error>;
    type Future = Future<C::Future, S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_command_and_services_ready(cx)
    }

    fn call(&mut self, request: crate::Request) -> Self::Future {
        self.route_request(request)
    }
}

#[derive(derive_new::new, Debug)]
pub(super) struct EnableService<S>(S);

#[derive(Debug, thiserror::Error)]
pub(super) enum Error<C, S> {
    #[error("control error")]
    Control(#[source] C),

    #[error(transparent)]
    Service(S),

    #[error("format error")]
    Format(#[from] format::Error),

    #[error("the request could not be handled")]
    UnhandledRequest,
}

impl<C, S> IsCanceledError for Error<C, S>
where
    C: IsCanceledError,
    S: IsCanceledError,
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
    C: TryFuture<Ok = Option<control::capabilities::CapabilitiesMap>>,
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

#[derive(Debug)]
enum Request {
    Control(control::Request),
    Service(service::Request),
}

impl Request {
    fn from_messaging(
        request: crate::Request,
    ) -> Result<Result<Self, crate::Request>, format::Error> {
        let request = match control::Request::try_from_messaging(request)? {
            Ok(request) => return Ok(Ok(Self::Control(request))),
            Err(request) => request,
        };
        let request = match service::Request::try_from_messaging(request) {
            Ok(request) => return Ok(Ok(Request::Service(request))),
            Err(request) => request,
        };
        Ok(Err(request))
    }
}
