use crate::{
    capabilities,
    channel::{Request as ChannelRequest, Response},
    format,
    message::{self, Action},
};
use async_trait::async_trait;
use sealed::sealed;

const AUTHENTICATE_SUBJECT: message::Subject = message::Subject::control(Action::new(8));

#[sealed]
#[async_trait]
pub(crate) trait ServiceExt {
    async fn authenticate(&mut self) -> Result<capabilities::Map, AuthenticateError>;
    async fn capabilities(
        &mut self,
        capabilities: capabilities::Map,
    ) -> Result<(), CapabilitiesError>;
}

#[sealed]
#[async_trait]
impl<S> ServiceExt for S
where
    S: tower::Service<ChannelRequest, Response = Response> + Send,
    S::Error: Into<Box<dyn std::error::Error>> + Send,
    S::Future: Send,
{
    async fn authenticate(&mut self) -> Result<capabilities::Map, AuthenticateError> {
        use tower::ServiceExt;
        let mut local_capabilities = capabilities::local();
        let request = ChannelRequest::call(AUTHENTICATE_SUBJECT, &local_capabilities)
            .map_err(AuthenticateError::FormatLocalCapabilities)?;
        let response = async { self.ready().await?.call(request).await }.await;
        let response = response.map_err(|err| AuthenticateError::Call(err.into()))?;
        let remote_capabilities = response
            .into_call_result()
            .map_err(|err| AuthenticateError::Call(err.into()))?;
        local_capabilities
            .resolve_minimums_against(&remote_capabilities, capabilities::reset_to_default)
            .check_required()
            .map_err(AuthenticateError::MissingRequiredCapabilities)?;
        Ok(local_capabilities)
    }

    async fn capabilities(
        &mut self,
        capabilities: capabilities::Map,
    ) -> Result<(), CapabilitiesError> {
        use tower::ServiceExt;
        let request = ChannelRequest::Capabilities(capabilities);
        let response = async { self.ready().await?.call(request).await }.await;
        response.map_err(|err| CapabilitiesError(err.into()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthenticateError {
    #[error("authentication call has failed")]
    Call(#[from] Box<dyn std::error::Error>),

    #[error("error serializing local capabilities")]
    FormatLocalCapabilities(#[source] format::Error),

    #[error("some required capabilities are missing")]
    MissingRequiredCapabilities(#[from] capabilities::ExpectedKeyValueError<bool>),
}

#[derive(Debug, thiserror::Error)]
#[error("capabilities request failed")]
pub(crate) struct CapabilitiesError(#[from] Box<dyn std::error::Error>);
