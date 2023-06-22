use super::{control, Service};
use crate::{
    format,
    messaging::{self, CallWithId, NotificationWithId},
    service::ToRequestId,
    Bytes, IsErrorCanceledTermination,
};
use futures::{ready, TryFuture};
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

#[derive(Debug)]
pub(super) struct Router<S> {
    control: control::Service,
    service: Option<S>,
    enable_service_receiver: Option<oneshot::Receiver<EnableService<S>>>,
}

/// Routes request between a control service and a client service.
impl<S> Router<S> {
    pub(super) fn new(control: control::Service) -> (Self, oneshot::Sender<EnableService<S>>) {
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

    pub(super) fn with_service_enabled(control: control::Service, service: S) -> Self {
        Self {
            control,
            service: Some(service),
            enable_service_receiver: None,
        }
    }

    fn enable_service(&mut self, service: S) {
        self.service.replace(service);
    }

    fn recv_enable_service(&mut self) {
        if let Some(enable_service) = self.enable_service_receiver.as_mut() {
            match enable_service.try_recv() {
                Ok(EnableService(service)) => {
                    self.enable_service(service);
                    self.enable_service_receiver = None
                }
                Err(oneshot::error::TryRecvError::Closed) => self.enable_service_receiver = None,
                Err(oneshot::error::TryRecvError::Empty) => (),
            }
        }
    }
}

// impl<'a, S> Route<'a, S> {
//     fn from_subject(router: &'a mut Router<S>, subject: crate::Subject) -> Option<Self> {
//         if let Some(service) = router.service.as_mut() {
//             if let Some(subject) = super::Subject::from_message(subject) {
//                 return Some(Self::Service { service, subject });
//             }
//         }

//         None
//     }
// }

impl<S> Service<CallWithId, NotificationWithId> for Router<S>
where
    S: Service<super::CallWithId, super::NotificationWithId>,
{
    type Error = Error<S::Error>;
    type CallFuture = CallFuture<S::CallFuture>;
    type NotifyFuture = NotifyFuture<S::NotifyFuture>;

    fn call(&mut self, call: CallWithId) -> Self::CallFuture {
        self.recv_enable_service();

        match control::Call::from_messaging(&call.inner) {
            Ok(Some(control_call)) => {
                return CallFuture::Control {
                    inner: self.control.call(control_call),
                }
            }
            Err(err) => return CallFuture::FormatError { error: Some(err) },
            _ => {}
        };

        if let Some(service) = self.service.as_mut() {
            if let Ok(call) = super::CallWithId::from_messaging(call) {
                return CallFuture::Service {
                    inner: service.call(call),
                };
            }
        }

        CallFuture::UnhandledRequest
    }

    fn notify(&mut self, notif_with_id: NotificationWithId) -> Self::NotifyFuture {
        self.recv_enable_service();

        let id = notif_with_id.to_request_id();
        let notif = match control::Notification::from_messaging(notif_with_id.into_inner()) {
            Ok(control_notif) => {
                return NotifyFuture::Control {
                    inner: self.control.notify(control_notif),
                }
            }
            Err(notif) => notif,
        };
        if let Some(service) = self.service.as_mut() {
            let notif_with_id = messaging::NotificationWithId::new(id, notif);
            if let Ok(notif) = super::NotificationWithId::from_messaging(notif_with_id) {
                return NotifyFuture::Service {
                    inner: service.notify(notif),
                };
            }
        }

        NotifyFuture::UnhandledRequest
    }
}

#[derive(derive_new::new, Debug)]
pub(super) struct EnableService<S>(S);

#[derive(Debug, thiserror::Error)]
pub(super) enum Error<S> {
    #[error("control error")]
    Control(#[source] control::Error),

    #[error(transparent)]
    Service(S),

    #[error("format error")]
    Format(#[from] format::Error),

    #[error("the request could not be handled")]
    UnhandledRequest,
}

impl<S> IsErrorCanceledTermination for Error<S>
where
    S: IsErrorCanceledTermination,
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
    #[project = CallFutureProj]
    #[must_use = "futures do nothing until polled"]
    pub(super) enum CallFuture<S> {
        Control {
            #[pin]
            inner: <control::Service as crate::Service<control::Call, control::Notification>>::CallFuture,
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

impl<S> Future for CallFuture<S>
where
    S: TryFuture<Ok = Bytes>,
{
    type Output = Result<Bytes, Error<S::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            CallFutureProj::Control { inner } => {
                let reply_payload = ready!(inner.try_poll(cx)).map_err(Error::Control)?;
                Poll::Ready(Ok(reply_payload))
            }
            CallFutureProj::Service { inner } => {
                let reply_payload = ready!(inner.try_poll(cx)).map_err(Error::Service)?;
                Poll::Ready(Ok(reply_payload))
            }
            CallFutureProj::FormatError { error } => match error.take() {
                Some(error) => Poll::Ready(Err(Error::Format(error))),
                None => Poll::Pending,
            },
            CallFutureProj::UnhandledRequest => Poll::Ready(Err(Error::UnhandledRequest)),
        }
    }
}

pin_project! {
    #[project = NotifyFutureProj]
    #[must_use = "futures do nothing until polled"]
    pub(super) enum NotifyFuture<S> {
        Control {
            #[pin]
            inner: <control::Service as crate::Service<control::Call, control::Notification>>::NotifyFuture
        },
        Service {
            #[pin]
            inner: S
        },
        UnhandledRequest,
    }
}

impl<S> Future for NotifyFuture<S>
where
    S: TryFuture<Ok = ()>,
{
    type Output = Result<(), Error<S::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            NotifyFutureProj::Control { inner } => {
                ready!(inner.try_poll(cx)).map_err(Error::Control)?;
                Poll::Ready(Ok(()))
            }
            NotifyFutureProj::Service { inner } => {
                ready!(inner.try_poll(cx)).map_err(Error::Service)?;
                Poll::Ready(Ok(()))
            }
            NotifyFutureProj::UnhandledRequest => Poll::Ready(Err(Error::UnhandledRequest)),
        }
    }
}
