use super::{
    capabilities::{self, MapExt},
    request,
};
use crate::{
    channel::{self, Channel},
    format,
    request::IsCanceled,
};
use futures::{future::BoxFuture, FutureExt};
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::Mutex;

#[derive(Debug)]
pub(in crate::session) struct Client {
    capabilities: Arc<Mutex<capabilities::Map>>,
}

impl Client {
    pub(crate) fn new() -> (Self, Service) {
        let capabilities = Arc::new(Mutex::new(capabilities::Map::new()));
        (
            Self {
                capabilities: Arc::clone(&capabilities),
            },
            Service { capabilities },
        )
    }

    pub(crate) async fn authenticate(
        &self,
        channel: &mut Channel,
    ) -> Result<&Self, AuthenticateError> {
        use tower::Service;
        let authenticate_request = request::Authenticate::new();
        let channel_request: channel::request::Call = authenticate_request
            .try_into()
            .map_err(AuthenticateError::SerializeLocalCapabilities)?;
        let response = async move { channel.ready().await?.call(channel_request).await };
        let remote_capabilities: capabilities::Map = match response.await {
            Ok(reply) => format::from_bytes(&reply)
                .map_err(AuthenticateError::DeserializeRemoteCapabilities)?,
            Err(channel::Error::Call(err)) => return Err(AuthenticateError::Service(err)),
            Err(channel::Error::Dispatch(err)) => {
                return Err(AuthenticateError::ChannelDispatch(err))
            }
        };
        let capabilities = remote_capabilities
            .check_intersect_with_local()
            .map_err(AuthenticateError::MissingRequiredCapabilities)?;
        *self.capabilities.lock().await = capabilities;
        Ok(self)
    }

    pub(in crate::session) async fn capabilities(&self) -> capabilities::Map {
        Arc::clone(&self.capabilities).lock_owned().await.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub(in crate::session) enum AuthenticateError {
    #[error("channel dispatch error")]
    ChannelDispatch(#[from] crate::client::DispatchError),

    #[error(transparent)]
    Service(#[from] crate::client::CallError),

    #[error("error serializing local capabilities")]
    SerializeLocalCapabilities(#[source] format::Error),

    #[error("error deserializing remote capabilities")]
    DeserializeRemoteCapabilities(#[source] format::Error),

    #[error("some required capabilities are missing")]
    MissingRequiredCapabilities(#[from] capabilities::ExpectedKeyValueError<bool>),
}

#[derive(Debug)]
pub(in crate::session) struct Service {
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
    ) -> Result<impl std::future::Future<Output = ()>, UpdateCapabilitiesError> {
        let capabilities = remote.clone().check_intersect_with_local()?;
        let self_capabilities = Arc::clone(&self.capabilities);
        Ok(async move {
            *self_capabilities.lock_owned().await = capabilities;
        })
    }
}

impl tower::Service<request::Request> for Service {
    type Response = Option<capabilities::Map>;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Option<capabilities::Map>, Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: request::Request) -> Self::Future {
        match req {
            request::Request::Authenticate(authenticate) => {
                let authenticate = self.call(authenticate);
                async move {
                    let result = authenticate.await?;
                    Ok(Some(result))
                }
                .boxed()
            }
            request::Request::UpdateCapabilities(update_capabilities) => {
                let update_capabilites = self.call(update_capabilities);
                async move {
                    update_capabilites.await?;
                    Ok(None)
                }
                .boxed()
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(in crate::session) enum Error {
    #[error(transparent)]
    AuthenticateRemote(#[from] AuthenticateRemoteError),

    #[error(transparent)]
    UpdateCapabilities(#[from] UpdateCapabilitiesError),
}

impl IsCanceled for Error {
    fn is_canceled(&self) -> bool {
        false
    }
}

impl tower::Service<request::Authenticate> for Service {
    type Response = capabilities::Map;
    type Error = AuthenticateRemoteError;
    type Future = BoxFuture<'static, Result<capabilities::Map, AuthenticateRemoteError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: request::Authenticate) -> Self::Future {
        self.authenticate_remote(request.into()).map(Ok).boxed()
    }
}

#[derive(Debug, thiserror::Error)]
pub(in crate::session) enum AuthenticateRemoteError {}

impl tower::Service<request::UpdateCapabilities> for Service {
    type Response = ();
    type Error = UpdateCapabilitiesError;
    type Future = BoxFuture<'static, Result<(), UpdateCapabilitiesError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: request::UpdateCapabilities) -> Self::Future {
        let update_capabilities = self.update_capabilities(&request.into());
        async move {
            update_capabilities?.await;
            Ok(())
        }
        .boxed()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("error updating capabilities")]
pub(in crate::session) struct UpdateCapabilitiesError(
    #[from] capabilities::ExpectedKeyValueError<bool>,
);
