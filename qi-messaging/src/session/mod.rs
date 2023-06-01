mod control;
pub mod request;

use crate::{capabilities, channel};
use futures::TryFuture;
pub use request::{Request, Response};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    spawn,
};
use tracing::info;

#[derive(Debug)]
pub struct Session {
    channel: channel::Channel,
    control_client: control::client::Client,
}

impl Session {
    pub async fn client<IO, Svc>(io: IO, service: Svc) -> Result<Self, ClientError>
    where
        IO: AsyncWrite + AsyncRead + Send + 'static,
        Svc: tower::Service<Request, Response = Response> + Send + 'static,
        Svc::Error: std::error::Error + Send,
        Svc::Future: Send,
    {
        let (control_client, control_service) = control::client::Client::new();
        let service = Service {
            control: control_service,
            service,
        };
        let (mut channel, dispatch) = channel::Channel::new(io, service);
        spawn(async {
            if let Err(err) = dispatch.await {
                info!(
                    error = &err as &dyn std::error::Error,
                    "session dispatch has returned an error"
                );
            }
        });

        control_client.authenticate(&mut channel).await?;
        Ok(Self {
            channel,
            control_client,
        })
    }

    pub async fn call() {
        todo!()
    }

    pub async fn post() {
        todo!()
    }

    pub async fn event() {
        todo!()
    }

    pub async fn capabilities(&self) -> capabilities::Map {
        self.control_client.capabilities().await
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ClientError(#[from] control::client::AuthenticateError);

#[derive(derive_new::new, Debug)]
struct Service<C, S> {
    control: C,
    service: S,
}

impl<C, S> tower::Service<channel::Request> for Service<C, S>
where
    C: tower::Service<control::Request, Response = control::Response>,
    S: tower::Service<Request, Response = Response>,
{
    type Response = channel::Response;
    type Error = ServiceError<S::Error>;
    type Future = ServiceFuture<S::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, req: channel::Request) -> Self::Future {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
enum ServiceError<E> {
    #[error(transparent)]
    Service(E),
}

#[derive(Debug)]
struct ServiceFuture<F>(F);

impl<F> std::future::Future for ServiceFuture<F>
where
    F: TryFuture,
{
    type Output = Result<channel::Response, ServiceError<F::Error>>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}

// impl Service<Request> for Session {
//     type Response = Option<Response>;
//     type Error = ServiceError;
//     type Future = RequestServiceFuture;

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         <Channel as Service<channel::Request>>::poll_ready(&mut self.channel, cx)
//             .map_err(ServiceError)
//     }

//     fn call(&mut self, req: Request) -> Self::Future {
//         todo!()
//     }
// }

// #[derive(Debug, thiserror::Error)]
// #[error(transparent)]
// pub struct ServiceError(channel::Error);
