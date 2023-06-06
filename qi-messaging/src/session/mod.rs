mod control;
mod router;
pub mod service;

use crate::{capabilities, channel::Channel, request::IsCanceled};
use bytes::Bytes;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    spawn,
};
use tracing::info;

#[derive(Debug)]
pub struct Session {
    channel: Channel,
    control_client: control::client::Client,
}

impl Session {
    pub async fn client<IO, Svc>(io: IO, service: Svc) -> Result<Self, ClientError>
    where
        IO: AsyncWrite + AsyncRead + Send + 'static,
        Svc: tower::Service<service::Request> + Send + 'static,
        Svc::Response: Into<Option<Bytes>>,
        Svc::Error: IsCanceled + std::fmt::Display + std::fmt::Debug + Sync + Send,
        Svc::Future: Send,
    {
        let (control_client, control_service) = control::client::Client::new();
        let service = self::router::Router::new(control_service, service);
        let (mut channel, dispatch) = Channel::new(io, service);
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

    pub async fn server<IO, Svc>(_io: IO, _service: Svc) -> Result<Self, ServerError> {
        todo!()
    }

    pub async fn capabilities(&self) -> capabilities::Map {
        self.control_client.capabilities().await
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ClientError(#[from] control::client::AuthenticateError);

#[derive(Debug, thiserror::Error)]
#[error("server error")]
pub struct ServerError;

// impl tower::Service<Request> for Session {
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
