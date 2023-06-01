use std::{
    future::{ready, Ready},
    task::{Context, Poll},
};

use super::{Error, Request, Response};
use crate::capabilities;
use tower::Service;

#[derive(Debug)]
pub(crate) struct Server {
    capabilities: capabilities::Map,
}

impl Server {
    fn authenticate_remote(&self, _capabilities: capabilities::Map) -> capabilities::Map {
        todo!()
    }

    fn update_capabilities(
        &mut self,
        remote: &capabilities::Map,
    ) -> Result<(), capabilities::ExpectedKeyValueError<bool>> {
        self.capabilities = capabilities::local_intersected_with(remote)?;
        Ok(())
    }
}

impl Service<Request> for Server {
    type Response = Response;
    type Error = Error;
    type Future = Ready<Result<Response, Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let response = match request {
            Request::Authenticate(capabilities) => {
                let response = self.authenticate_remote(capabilities);
                Ok(Response(Some(response)))
            }
            Request::UpdateCapabilities(remote) => self
                .update_capabilities(&remote)
                .map_err(Error::UpdateCapabilities)
                .map(|()| Response(None)),
        };
        ready(response)
    }
}
