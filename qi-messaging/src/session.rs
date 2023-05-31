use crate::{
    capabilities,
    channel::{self, Channel, Request, Response},
    control,
    request::Request as MessagingRequest,
};
use futures::FutureExt;
use std::{
    fmt::Debug,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    spawn,
};
use tower::Service;
use tracing::info;

#[derive(Debug)]
pub struct Session {
    channel: Channel,
    capabilities: capabilities::Map,
}

impl Session {
    pub async fn new<IO, Svc>(io: IO, service: Svc) -> Result<Session, Error>
    where
        IO: AsyncWrite + AsyncRead + Send + 'static,
        Svc: Service<MessagingRequest, Response = Response> + Send + 'static,
        Svc::Future: Send,
        Svc::Error: std::error::Error + Send,
    {
        use crate::control::ServiceExt;
        let (mut channel, dispatch) = Channel::new(io, service);
        let capabilities = channel.authenticate().await?;
        spawn(async {
            if let Err(err) = dispatch.await {
                info!(
                    error = &err as &dyn std::error::Error,
                    "session dispatch has returned an error"
                );
            }
        });
        Ok(Self {
            channel,
            capabilities,
        })
    }

    pub fn capabilities(&self) -> &capabilities::Map {
        &self.capabilities
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(#[from] control::AuthenticateError);

impl Service<Request> for Session {
    type Response = Response;
    type Error = ServiceError;
    type Future = ServiceFuture;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Channel as Service<Request>>::poll_ready(&mut self.channel, cx).map_err(ServiceError)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        ServiceFuture(self.channel.call(req))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ServiceError(channel::Error);

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub struct ServiceFuture(channel::Future);

impl std::future::Future for ServiceFuture {
    type Output = Result<Response, ServiceError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_unpin(cx).map_err(ServiceError)
    }
}
