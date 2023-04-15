use crate::{
    capabilities,
    channel::{Call, CallEndError, Channel, RequestStartError},
    connection::Connection,
    dispatch::Dispatch,
    server, CallResult, Params,
};
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Debug)]
pub struct Session {
    channel: Channel,
    capabilities: capabilities::Map,
}

impl Session {
    pub async fn call<T, R>(&self, params: Params<T>) -> Result<Call<R>, RequestStartError>
    where
        T: serde::Serialize,
    {
        self.channel.call(params).await
    }

    pub async fn post<T>(&self, params: Params<T>) -> Result<(), RequestStartError>
    where
        T: serde::Serialize,
    {
        self.channel.post(params).await
    }

    pub async fn event<T>(&self, params: Params<T>) -> Result<(), RequestStartError>
    where
        T: serde::Serialize,
    {
        self.channel.event(params).await
    }
}

pub fn connect<IO>(io: IO) -> (impl Future<Output = Result<Session, Error>>, Connection<IO>)
where
    IO: AsyncRead + AsyncWrite,
{
    let (dispatch, orders) = Dispatch::new();
    let connection = Connection::new(io, dispatch);
    let channel = Channel::new(orders);

    let session = async move {
        let mut capabilities = capabilities::local();

        use server::ServerCall;
        let params = Params::builder()
            .server_authenticate()
            .argument(&capabilities)
            .build();
        let authenticate = channel.call(params).await?;
        let remote_capabilities = match authenticate.await? {
            CallResult::Ok(capabilities) => capabilities,
            CallResult::Err(_error) => todo!(),
            CallResult::Canceled => todo!(),
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

impl From<RequestStartError> for Error {
    fn from(_: RequestStartError) -> Self {
        todo!()
    }
}

impl From<CallEndError> for Error {
    fn from(_: CallEndError) -> Self {
        todo!()
    }
}
