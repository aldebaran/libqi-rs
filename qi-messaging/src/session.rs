pub mod as_service;
mod control;
mod router;
pub mod service;
use std::task::{Context, Poll};

use bytes::Bytes;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    spawn,
};
use tracing::{debug, debug_span, info, Instrument};

use crate::{
    capabilities,
    channel::{self, Channel},
    IsCanceledError,
};

#[derive(Debug)]
pub struct Session {
    channel: Channel,
    control: control::Control,
}

impl Session {
    pub async fn capabilities(&self) -> capabilities::CapabilitiesMap {
        self.control.capabilities().await
    }

    fn poll_channel_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), channel::Error>> {
        <Channel as tower::Service<channel::Request>>::poll_ready(&mut self.channel, cx)
    }

    pub async fn ready(&mut self) -> Result<&mut Self, as_service::Error> {
        tower::ServiceExt::<as_service::Request>::ready(self).await
    }
}

fn spawn_channel<IO, Svc>(io: IO, service: Svc) -> Channel
where
    IO: AsyncWrite + AsyncRead + Send + 'static,
    Svc: tower::Service<crate::Request> + Send + 'static,
    Svc::Response: Into<Option<Bytes>> + Send + std::fmt::Debug + 'static,
    Svc::Error: IsCanceledError + std::fmt::Display + std::fmt::Debug + Send + 'static,
    Svc::Future: Send,
{
    let (channel, dispatch) = Channel::new(io, service);
    let dispatch = dispatch.instrument(debug_span!("dispatch"));
    spawn(async move {
        if let Err(err) = dispatch.await {
            info!(
                error = &err as &dyn std::error::Error,
                "session dispatch has returned an error"
            );
        }
    });
    channel
}

pub async fn connect<IO, Svc>(io: IO, service: Svc) -> Result<Session, ConnectError>
where
    IO: AsyncWrite + AsyncRead + Send + 'static,
    Svc: tower::Service<service::Request> + Send + 'static,
    Svc::Response: Into<Option<Bytes>>,
    Svc::Error: IsCanceledError + std::fmt::Display + std::fmt::Debug + Sync + Send,
    Svc::Future: Send,
{
    // As a client, we can enable the service in the router right away.
    let (control, control_service) = control::Control::new_with_service();
    let router = router::Router::new_service_enabled(control_service, service);
    let mut channel = spawn_channel(io, router);
    control.authenticate_to_remote(&mut channel).await?;
    Ok(Session { channel, control })
}

#[doc(inline)]
pub use {
    crate::client::{CallError, DispatchError},
    control::{AuthenticateToRemoteError as ConnectError, VerifyAuthenticationResultError},
};

pub async fn listen<IO, Svc>(io: IO, service: Svc) -> Result<Session, ListenError>
where
    IO: AsyncWrite + AsyncRead + Send + 'static,
    Svc: tower::Service<service::Request> + Send + 'static,
    Svc::Response: Into<Option<Bytes>>,
    Svc::Error: IsCanceledError + std::fmt::Display + std::fmt::Debug + Sync + Send,
    Svc::Future: Send,
{
    // As a server, we first have to create the router, then wait for a successful
    // authentication to enable access to the service.
    let (mut control, control_service) = control::Control::new_with_service();
    let (router, router_command_sender) = router::Router::new(control_service);
    let channel = spawn_channel(io, router);

    control.remote_authentication().await?;
    if router_command_sender
        .send(router::EnableService::new(service))
        .is_err()
    {
        debug!("error sending a command to the session router service, the command is discarded.");
    }
    Ok(Session { channel, control })
}

#[doc(inline)]
pub use control::RemoteAuthenticationError as ListenError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::Request;
    use bytes::Bytes;
    use futures::{future, FutureExt, TryFutureExt};
    use qi_format::from_bytes;
    use tokio::{io, join};
    use tower::{util::BoxService, Service, ServiceExt};

    #[derive(Debug, thiserror::Error)]
    enum Error<E> {
        #[error("format error")]
        Format(#[from] qi_format::Error),

        #[error(transparent)]
        Service(E),
    }

    impl<E> IsCanceledError for Error<E>
    where
        E: IsCanceledError,
    {
        fn is_canceled(&self) -> bool {
            match self {
                Error::Format(_) => false,
                Error::Service(s) => s.is_canceled(),
            }
        }
    }

    fn on_call<F, Fut, T, U, E>(mut f: F) -> BoxService<Request, Option<Bytes>, Error<E>>
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<U, E>> + Send + 'static,
        T: serde::de::DeserializeOwned,
        U: serde::Serialize,
        E: Send + 'static,
    {
        let service = move |request: Request| {
            let f = &mut f;
            match request {
                Request::Call(call) => match call.value() {
                    Ok(input) => f(input)
                        .map_err(Error::Service)
                        .map(|output| {
                            let output = output?;
                            let reply = qi_format::to_bytes(&output)?;
                            Ok(Some(reply))
                        })
                        .boxed(),
                    Err(err) => future::err(Error::Format(err)).boxed(),
                },
                _ => unreachable!(),
            }
        };
        tower::service_fn(service).boxed()
    }

    fn to_async<F, T, U>(f: F) -> impl Fn(T) -> future::Ready<U>
    where
        F: Fn(T) -> U,
    {
        move |input| future::ready(f(input))
    }

    fn to_try<F, T, U>(f: F) -> impl Fn(T) -> Result<U, std::convert::Infallible>
    where
        F: Fn(T) -> U,
    {
        move |input| Ok(f(input))
    }

    fn add_to_string((a, b): (i32, i32)) -> String {
        (a + b).to_string()
    }

    fn sum(elems: Vec<i32>) -> i32 {
        elems.iter().sum()
    }

    struct TestSessionPair {
        client: Session,
        server: Session,
    }

    impl TestSessionPair {
        async fn new() -> Self {
            let (io_client, io_server) = io::duplex(256);
            let client = connect(io_client, on_call(to_async(to_try(sum)))).map(Result::unwrap);
            let server =
                listen(io_server, on_call(to_async(to_try(add_to_string)))).map(Result::unwrap);
            let (client, server) = join!(client, server);
            Self { client, server }
        }
    }

    fn any_service_subject() -> service::Subject {
        service::Subject::new(
            service::Service::new(1),
            service::Object::new(1),
            service::Action::new(1),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_session_pair_call() {
        let TestSessionPair {
            mut client,
            mut server,
        } = TestSessionPair::new().await;

        let subject = any_service_subject();
        let reply = client
            .ready()
            .await
            .unwrap()
            .call(as_service::Call::with_value(subject, &(12, -49)).unwrap())
            .await
            .unwrap();
        let value: String = from_bytes(&reply).unwrap();
        assert_eq!(value, "-37");

        let reply = server
            .ready()
            .await
            .unwrap()
            .call(
                as_service::Call::with_value(subject, &vec![32, 2893, -123, 3287, 0, -38293])
                    .unwrap(),
            )
            .await
            .unwrap();
        let value: i32 = from_bytes(&reply).unwrap();
        assert_eq!(value, -32204);
    }
}
