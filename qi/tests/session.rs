use assert_matches::assert_matches;
use futures::{channel::mpsc, SinkExt, StreamExt};
use qi::{
    messaging::{self, message, Message},
    session::{self, authentication},
    value::KeyDynValueMap,
    HandlerError,
};
use qi_messaging::Body;
use serde_json as json;
use std::{
    collections::VecDeque,
    convert::Infallible,
    future::{ready, Future},
};
use tokio::spawn;

#[derive(Clone, Copy)]
struct DummyHandler;

impl messaging::Handler<JsonBody> for DummyHandler {
    type Error = HandlerError;

    async fn call(
        &self,
        _address: message::Address,
        value: JsonBody,
    ) -> Result<JsonBody, Self::Error> {
        Ok(value)
    }

    fn fire_and_forget(
        &self,
        _address: message::Address,
        _request: message::FireAndForget<JsonBody>,
    ) -> impl Future<Output = ()> + Send {
        ready(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsonBody(json::Value);

impl messaging::Body for JsonBody {
    type Error = json::Error;
    type Data = VecDeque<u8>;

    fn from_bytes(bytes: bytes::Bytes) -> Result<Self, Self::Error> {
        json::from_slice(&bytes).map(Self)
    }

    fn into_data(self) -> Result<Self::Data, Self::Error> {
        json::to_vec(&self.0).map(Into::into)
    }

    fn serialize<T>(value: &T) -> Result<Self, Self::Error>
    where
        T: serde::Serialize,
    {
        json::to_value(value).map(Self)
    }

    fn deserialize_seed<'de, T>(&'de self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.0.clone())
    }
}

/// The server session receives an authentication request with incompatible capabilities.
/// It is expected to:
///   1. Reply to the request with an error.
///   2. Close the connection.
#[tokio::test]
async fn server_sends_back_error_on_client_bad_capabilities() {
    // 0.1: start the server session
    let (mut send_to_server, server_recv) = mpsc::unbounded();
    let (server_send, mut recv_from_server) = mpsc::unbounded();
    let task = spawn(session::Session::serve_client(
        server_recv.map(Ok::<_, Infallible>),
        server_send.sink_map_err(qi_messaging::Error::link_lost),
        authentication::PermissiveAuthenticator,
        DummyHandler,
    ));

    // 0.2: start the request
    send_to_server
        .send(Message::Call {
            id: message::Id(0),
            address: session::control::AUTHENTICATE_ADDRESS,
            value: JsonBody::serialize(&{
                let mut map = KeyDynValueMap::new();
                map.set("RemoteCancelableCalls", true);
                map.set("ObjectPtrUID", true);
                map.set("RelativeEndpointURI", false); // A required capabilities is set to false.
                map
            })
            .unwrap(),
        })
        .await
        .unwrap();

    // 1.
    let response = recv_from_server.next().await.unwrap();
    assert_matches!(
        response,
        Message::Error {
            id: _,
            address: session::control::AUTHENTICATE_ADDRESS,
            error
        } => {
            assert!(error.contains("unexpected capability value"))
        }
    );

    // 2.
    let () = task.await.unwrap();
}

#[tokio::test]
async fn client_receives_bad_capabilities() {
    todo!()
}

#[tokio::test]
async fn client_bad_authentication() {
    todo!()
}

#[tokio::test]
async fn server_bad_authentication() {
    todo!()
}
