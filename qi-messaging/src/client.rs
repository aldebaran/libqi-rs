pub use crate::session::Connection;
use crate::{
    capabilities,
    message_types::Response,
    server,
    session::{self, Session},
};
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};

pub fn connect<IO>(io: IO) -> (impl Future<Output = Result<Client, Error>>, Connection<IO>)
where
    IO: 'static + AsyncRead + AsyncWrite + Unpin,
{
    let (bare_session, connection) = session::make_bare(io);
    let client = async move {
        let mut capabilities = capabilities::local();

        use server::ToServer;
        let auth_response = bare_session
            .call()
            .to_server()
            .authenticate()
            .argument(&capabilities)
            .send()?;

        let remote_capabilities = match auth_response.await? {
            Response::Reply(value) => value.into_value(),
            Response::Error(_error) => todo!(),
            Response::Canceled(_canceled) => todo!(),
        };

        capabilities
            .update_to_minimums_with(&remote_capabilities, capabilities::default_capability);
        Ok(Client {
            bare_session,
            capabilities,
        })
    };
    (client, connection)
}

#[derive(Debug)]
pub struct Client {
    bare_session: session::Bare,
    capabilities: capabilities::Map,
}

impl Session for Client {
    fn call<R>(&self) -> session::CallRequestBuilder<R> {
        self.bare_session.call()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl From<session::SendCallRequestError> for Error {
    fn from(_: session::SendCallRequestError) -> Self {
        todo!()
    }
}
