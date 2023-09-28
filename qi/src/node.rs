use crate::{
    messaging::{self, session, CallResult},
    object,
    service_directory::{self, BoxServiceDirectory},
    transport::{self, Transport},
    Uri,
};
use futures::future::BoxFuture;
use tokio::spawn;
use tracing::{instrument, trace, trace_span, Instrument};

pub struct Node {
    service_directory: BoxServiceDirectory<'static>,
}

impl Node {
    #[instrument(level = "trace", skip_all, ret)]
    pub async fn to_namespace(uri: Uri) -> CallResult<Self, ToNamespaceError> {
        let transport = Transport::connect(uri)
            .await
            .map_err(ToNamespaceError::TransportFromUri)?;
        let service = MessagingService;
        let (session_client, session) = session::connect(transport, service);

        spawn(
            async move {
                if let Err(err) = session.await {
                    trace!(
                        error = &err as &dyn std::error::Error,
                        "session terminated with an error"
                    )
                }
            }
            .instrument(trace_span!(parent: None, "dispatch")),
        );

        let session_client = session_client
            .await
            .map_err(ToNamespaceError::SessionConnect)?;
        let sd_client = service_directory::Client::connect(session_client)
            .await
            .map_err(|err| err.map_err(ToNamespaceError::ConnectServiceDirectoryClient))?;
        let service_directory = Box::new(sd_client);

        Ok(Self { service_directory })
    }

    pub fn service_directory(&self) -> &BoxServiceDirectory<'static> {
        &self.service_directory
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Node")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToNamespaceError {
    #[error("failed to create a transport for this URI")]
    TransportFromUri(#[from] transport::ConnectFromUriError),

    #[error(transparent)]
    SessionConnect(#[from] session::ConnectError),

    #[error("failed to connect the client of the service directory main object")]
    ConnectServiceDirectoryClient(#[from] object::client::ConnectError),
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {}

#[derive(Debug)]
struct MessagingService;

impl messaging::Service<session::CallWithId, session::NotificationWithId> for MessagingService {
    type CallReply = ();
    type Error = MessagingServiceError;
    type CallFuture = BoxFuture<'static, CallResult<Self::CallReply, Self::Error>>;
    type NotifyFuture = BoxFuture<'static, Result<(), Self::Error>>;

    fn call(&mut self, call: session::CallWithId) -> Self::CallFuture {
        todo!()
    }

    fn notify(&mut self, notif: session::NotificationWithId) -> Self::NotifyFuture {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("messaging service error")]
struct MessagingServiceError;
