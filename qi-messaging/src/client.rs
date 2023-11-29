use crate::{
    endpoint::{self, ClientRequest},
    message, Error, Service,
};
use bytes::Bytes;
use futures::{future::BoxFuture, FutureExt, SinkExt};
use tokio::sync::oneshot;
use tokio_util::sync::{CancellationToken, PollSender};

#[derive(Clone)]
pub struct Client {
    requests: PollSender<endpoint::ClientRequest>,
}

impl Client {
    pub(crate) fn new(requests: PollSender<ClientRequest>) -> Self {
        Self { requests }
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Client")
    }
}

impl Service for Client {
    fn call(&self, call: Call) -> BoxFuture<'static, Result<Bytes, Error>> {
        let mut requests = self.requests.clone();
        async move {
            let (response_sender, response_receiver) = oneshot::channel();
            let cancel_token = CancellationToken::new();
            let call = ClientRequest::Call {
                call,
                cancel_token: cancel_token.clone(),
                response_sender,
            };
            requests.send(call).await?;
            let _drop_guard = cancel_token.drop_guard();
            let response = response_receiver.await??;
            Ok(response)
        }
        .boxed()
    }

    fn post(&self, post: Post) -> BoxFuture<'static, Result<(), Error>> {
        let mut requests = self.requests.clone();
        async move {
            requests.send(ClientRequest::Post(post)).await?;
            Ok(())
        }
        .boxed()
    }

    fn event(&self, event: Event) -> BoxFuture<'static, Result<(), Error>> {
        let mut requests = self.requests.clone();
        async move {
            requests.send(ClientRequest::Event(event)).await?;
            Ok(())
        }
        .boxed()
    }
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Call {
    pub(crate) address: message::Address,
    pub(crate) value: Bytes,
}

impl Call {
    pub fn new(address: message::Address, value: Bytes) -> Self {
        Self { address, value }
    }

    pub fn address(&self) -> message::Address {
        self.address
    }

    pub fn value(&self) -> &Bytes {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut Bytes {
        &mut self.value
    }
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Post {
    pub(crate) address: message::Address,
    pub(crate) value: Bytes,
}

impl Post {
    pub fn new(address: message::Address, value: Bytes) -> Self {
        Self { address, value }
    }

    pub fn address(&self) -> message::Address {
        self.address
    }

    pub fn value(&self) -> &Bytes {
        &self.value
    }
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Event {
    pub(crate) address: message::Address,
    pub(crate) value: Bytes,
}

impl Event {
    pub fn new(address: message::Address, value: Bytes) -> Self {
        Self { address, value }
    }

    pub fn address(&self) -> message::Address {
        self.address
    }

    pub fn value(&self) -> &Bytes {
        &self.value
    }
}
