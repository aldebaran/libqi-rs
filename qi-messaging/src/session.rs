use crate::{
    call, capabilities,
    channel::{Call, CallEndError, CallStartError, Channel},
    connection::Connection,
    server,
};
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Debug)]
pub struct Session {
    channel: Channel,
    capabilities: capabilities::Map,
}

impl Session {
    pub async fn call<T, R>(&self, params: call::Params<T>) -> Result<Call<R>, CallStartError>
    where
        T: serde::Serialize,
    {
        self.channel.call(params).await
    }
}

pub fn connect<IO>(io: IO) -> (impl Future<Output = Result<Session, Error>>, Connection<IO>)
where
    IO: AsyncRead + AsyncWrite,
{
    let (connection, dispatch_orders) = Connection::new(io);
    let channel = Channel::new(dispatch_orders);

    let session = async move {
        let mut capabilities = capabilities::local();

        use server::ServerCall;
        let params = call::Params::builder()
            .server_authenticate()
            .argument(&capabilities)
            .build();
        let authenticate = channel.call(params).await?;
        let remote_capabilities = match authenticate.await? {
            call::Result::Ok(capabilities) => capabilities,
            call::Result::Err(_error) => todo!(),
            call::Result::Canceled => todo!(),
        };

        capabilities
            .resolve_minimums_against(&remote_capabilities, capabilities::default_capability);
        Ok(Session {
            channel,
            capabilities,
        })
    };
    (session, connection)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl From<CallStartError> for Error {
    fn from(_: CallStartError) -> Self {
        todo!()
    }
}

impl From<CallEndError> for Error {
    fn from(_: CallEndError) -> Self {
        todo!()
    }
}
