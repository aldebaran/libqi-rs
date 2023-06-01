use super::{Error, Request, Response};
use crate::{
    capabilities,
    channel::{self, Channel},
    format,
    request::CallError as MessagingCallError,
};
use futures::{future::BoxFuture, ready, FutureExt};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Client {
    capabilities: Arc<Mutex<capabilities::Map>>,
}

impl Client {
    pub(crate) fn new() -> (Self, Service) {
        let capabilities = Arc::new(Mutex::new(capabilities::Map::new()));
        (
            Self {
                capabilities: capabilities.clone(),
            },
            Service { capabilities },
        )
    }

    pub(crate) async fn authenticate(
        &self,
        channel: &mut Channel,
    ) -> Result<&Self, AuthenticateError> {
        use tower::{Service, ServiceExt};
        let mut capabilities = capabilities::local().clone();
        let request = Request::Authenticate(capabilities.clone());
        let request = request
            .try_into_channel_request()
            .map_err(AuthenticateError::FormatLocalCapabilities)?;
        let response = async { channel.ready().await?.call(request).await }
            .await
            .map_err(AuthenticateError::Channel)?;
        let remote_capabilities = response
            .into_call_result()
            .map_err(AuthenticateError::Call)?;
        capabilities
            .intersect(&remote_capabilities)
            .check_required()
            .map_err(AuthenticateError::MissingRequiredCapabilities)?;
        *self.capabilities.lock().await = capabilities;
        Ok(self)
    }

    pub async fn capabilities(&self) -> capabilities::Map {
        self.capabilities.clone().lock_owned().await.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthenticateError {
    #[error("channel error")]
    Channel(#[from] channel::Error),

    #[error("the call request has resulted in an error: {0}")]
    Call(#[from] MessagingCallError),

    #[error("error serializing local capabilities")]
    FormatLocalCapabilities(#[source] format::Error),

    #[error("some required capabilities are missing")]
    MissingRequiredCapabilities(#[from] capabilities::ExpectedKeyValueError<bool>),
}

#[derive(Debug)]
pub(crate) struct Service {
    capabilities: Arc<Mutex<capabilities::Map>>,
}

impl Service {
    // Handle an authentication request as a client, which is unusual but does not matter.
    // Always returns an authentication success.
    fn authenticate_remote(
        &self,
        _capabilities: capabilities::Map,
    ) -> impl std::future::Future<Output = capabilities::Map> {
        async { todo!() }
    }

    fn update_capabilities(
        &self,
        remote: &capabilities::Map,
    ) -> Result<impl std::future::Future<Output = ()>, capabilities::ExpectedKeyValueError<bool>>
    {
        let capabilities = capabilities::local_intersected_with(remote)?;
        let self_capabilities = self.capabilities.clone();
        Ok(async move {
            *self_capabilities.lock_owned().await = capabilities;
        })
    }
}

impl tower::Service<Request> for Service {
    type Response = Response;
    type Error = Error;
    type Future = Future;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request) -> Self::Future {
        match request {
            Request::Authenticate(capabilities) => {
                let authenticate = self.authenticate_remote(capabilities);
                Future::Authenticate(authenticate.boxed())
            }
            Request::UpdateCapabilities(remote) => {
                let update = self.update_capabilities(&remote);
                Future::UpdateCapabilities(update.map(|f| f.boxed()).map_err(Some))
            }
        }
    }
}
#[must_use = "futures do nothing until polled"]
pub(crate) enum Future {
    Authenticate(BoxFuture<'static, capabilities::Map>),
    UpdateCapabilities(
        Result<BoxFuture<'static, ()>, Option<capabilities::ExpectedKeyValueError<bool>>>,
    ),
}

impl std::future::Future for Future {
    type Output = Result<Response, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(match self.get_mut() {
            Self::Authenticate(authenticate) => {
                let capabilities = ready!(authenticate.poll_unpin(cx));
                Ok(Response(Some(capabilities)))
            }
            Self::UpdateCapabilities(Ok(update)) => {
                ready!(update.poll_unpin(cx));
                Ok(Response(None))
            }
            Self::UpdateCapabilities(Err(err)) => match err.take() {
                Some(err) => Err(Error::UpdateCapabilities(err)),
                None => return Poll::Pending,
            },
        })
    }
}
