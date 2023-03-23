use crate::{
    capabilities, server,
    session::{self, Call, CallBuilder, Inner, Response, Session},
    stream::Stream,
};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

pub async fn connect<IO>(io: IO) -> Result<Client, Error>
where
    IO: AsyncRead + AsyncWrite + Sync,
{
    let stream = Stream::new(io);
    let over_stream = Inner::new(stream);
    let client = Client::connect(over_stream).await?;
    Ok(client)
}

#[derive(Debug)]
pub struct Client<S> {
    session: S,
    capabilities: capabilities::Map,
}

impl<S> Client<S>
where
    S: Session,
{
    async fn connect(session: S) -> Result<Self, Error> {
        let local_capabilities = capabilities::local();

        use server::ToServer;
        let authenticate = session
            .create_call()
            .to_server()
            .authenticate()
            .argument(&local_capabilities)
            .build();

        let response = session.send_call_request(authenticate).await?;

        let remote_capabilities = match response {
            Response::Ok(value) => value.into_value(),
            Response::Error => todo!(),
            Response::Canceled => todo!(),
        };

        let capabilities = local_capabilities.merged_with(remote_capabilities);

        Ok(Self {
            session,
            capabilities,
        })
    }
}

#[async_trait]
impl<S> Session for Client<S>
where
    S: Session + Sync,
{
    fn create_call(&self) -> CallBuilder {
        self.session.create_call()
    }

    async fn send_call_request<T, R>(&self, call: Call<T>) -> Result<Response<R>, session::Error>
    where
        T: Send,
    {
        let response = self.session.send_call_request(call);
        response.await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl From<session::Error> for Error {
    fn from(_: session::Error) -> Self {
        todo!()
    }
}
