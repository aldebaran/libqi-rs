use crate::{
    capabilities::{self, CapabilityMap},
    message::{Call, CallBuilder, Capability},
    message::{IntoCapabilityError, IntoMessageError},
    stream::{DecodeError, EncodeError, Stream},
};
use futures::{SinkExt, StreamExt};

#[derive(Debug)]
pub struct Channel<T> {
    stream: Stream<T>,
    capabilities: CapabilityMap,
}

fn make_capability<T>(
    stream: &mut Stream<T>,
    capabilities: capabilities::CapabilityMap,
) -> Capability {
    Capability::new(stream.next_message_id(), capabilities)
}

impl<T> Channel<T>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn new(io: T) -> Result<Self, Error> {
        let mut stream = Stream::new(io);

        // Send capabilities
        let local_capabilities = capabilities::local_capabilities();
        let capability = make_capability(&mut stream, local_capabilities.clone());
        stream.send(capability.into_message()?).await?;

        // Receive capabilities
        let reply = stream.next().await.ok_or(Error::EndOfStream)??;
        let capability: Capability = reply.try_into()?;
        let remote_capabilities = capability.capabilities;

        let capabilities = local_capabilities.merged_with(&remote_capabilities);
        Ok(Self {
            stream,
            capabilities,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("end of stream")]
    EndOfStream,

    #[error("failed to encode a message: {0}")]
    MessageEncoding(#[from] EncodeError),

    #[error("failed to decode a message: {0}")]
    MessageDecoding(#[from] DecodeError),

    #[error("failed to convert into a message: {0}")]
    IntoMessage(#[from] IntoMessageError),

    #[error("failed to convert a message into a capability message: {0}")]
    IntoCapability(#[from] IntoCapabilityError),
}

impl<T> Channel<T> {
    fn call_request(&mut self) -> CallBuilder {
        Call::builder(self.stream.next_message_id())
    }
}
