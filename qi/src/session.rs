pub(crate) mod authentication;
mod capabilities;
mod control;
mod message_body;
mod reference;

pub use self::reference::Reference;
use crate::{
    messaging::{self, message, CapabilitiesMap},
    Authenticator, Error, PermissiveAuthenticator, Result,
};
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Sink, Stream, StreamExt, TryFutureExt,
};
use message_body::BinaryValue;
use std::{future::Future, pin::pin};
use tokio::{select, sync::watch};

#[derive(Clone, Debug)]
pub(crate) struct Session {
    uid: Uid,
    capabilities: watch::Receiver<Option<CapabilitiesMap<'static>>>,
    client: messaging::Client<BinaryValue, BinaryValue>,
}

impl Session {
    pub(crate) async fn connect<CallHandler, OnewaySink>(
        address: messaging::Address,
        credentials: authentication::Parameters<'_>,
        call_handler: CallHandler,
        oneway_sink: OnewaySink,
    ) -> Result<(Self, impl Future<Output = Result<()>>)>
    where
        CallHandler:
            tower::Service<(message::Address, BinaryValue), Error = Error, Response = BinaryValue>,
        OnewaySink: Sink<(message::Address, message::OnewayRequest<BinaryValue>), Error = Error>,
    {
        let (capabilities_sender, capabilities_receiver) = watch::channel(Default::default());
        let (controlled_handler, controlled_oneway_sink) = control::wrap(
            call_handler,
            oneway_sink,
            Box::new(PermissiveAuthenticator),
            capabilities_sender.clone(),
            true,
        );
        let (messages_stream, messages_sink) = messaging::channel::connect(address).await?;
        let (mut client, connection) = messaging::endpoint::start(
            messages_stream,
            messages_sink,
            controlled_handler,
            controlled_oneway_sink,
        );
        control::authenticate_to_remote(&mut client, credentials, &capabilities_sender).await?;
        Ok((
            Session {
                uid: Uid::new(),
                capabilities: capabilities_receiver,
                client,
            },
            connection.map_err(Into::into),
        ))
    }

    pub(crate) async fn server<Auth, CallHandler, OnewaySink>(
        address: messaging::Address,
        authenticator: Auth,
        call_handler: CallHandler,
        oneway_sink: OnewaySink,
    ) -> Result<(impl Future<Output = ()>, Vec<messaging::Address>)>
    where
        Auth: Authenticator + Clone,
        CallHandler: tower::Service<(message::Address, BinaryValue), Error = Error, Response = BinaryValue>
            + Clone,
        OnewaySink:
            Sink<(message::Address, message::OnewayRequest<BinaryValue>), Error = Error> + Clone,
    {
        let (clients, endpoints) = messaging::channel::serve(address).await?;
        let server = async move {
            let mut clients = pin!(clients.fuse());
            let mut sessions = FuturesUnordered::new();
            loop {
                select! {
                    Some((messages_stream, messages_sink, _address)) = clients.next(), if !clients.is_terminated() => {
                        sessions.push(Self::serve(messages_stream, messages_sink, authenticator.clone(), call_handler.clone(), oneway_sink.clone()));
                    }
                    _res = sessions.next(), if !sessions.is_terminated() => {
                        // nothing
                    }
                    else => {
                        break
                    }
                }
            }
        };
        Ok((server, endpoints))
    }

    async fn serve<Auth, MsgStream, MsgSink, CallHandler, OnewaySink>(
        messages_stream: MsgStream,
        messages_sink: MsgSink,
        authenticator: Auth,
        call_handler: CallHandler,
        oneway_sink: OnewaySink,
    ) where
        MsgStream:
            Stream<Item = std::result::Result<messaging::Message<BinaryValue>, messaging::Error>>,
        MsgSink: Sink<messaging::Message<BinaryValue>, Error = messaging::Error>,
        Auth: Authenticator + 'static,
        CallHandler:
            tower::Service<(message::Address, BinaryValue), Error = Error, Response = BinaryValue>,
        OnewaySink: Sink<(message::Address, message::OnewayRequest<BinaryValue>), Error = Error>,
    {
        let (capabilities_sender, capabilities_receiver) = watch::channel(Default::default());
        let (controlled_handler, controlled_oneway_sink) = control::wrap(
            call_handler,
            oneway_sink,
            Box::new(authenticator),
            capabilities_sender,
            true,
        );
        let (mut client, connection) = messaging::endpoint::start(
            messages_stream,
            messages_sink,
            controlled_handler,
            controlled_oneway_sink,
        );
        let _session = Self {
            uid: Uid::new(),
            capabilities: capabilities_receiver,
            client,
        }
    }

    pub(crate) fn messaging_client(&mut self) -> &mut messaging::Client<T, R> {
        &mut self.client
    }

    pub fn uid(&self) -> Uid {
        self.uid.clone()
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "crate::value", transparent)]
pub struct Uid(String);

impl Uid {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for Uid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Uid {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self::from_string(s.to_owned()))
    }
}

// #[derive(Debug)]
// struct Service<S, A> {
//     inner: S,
//     control: control::Control<A>,
// }

// impl<S, A> Service<S, A> {
//     fn server(
//         inner: S,
//         authenticator: A,
//         capabilities: Arc<Mutex<Option<CapabilitiesMap>>>,
//     ) -> Self {
//         Self::new(inner, authenticator, capabilities, false)
//     }

//     fn new(
//         inner: S,
//         authenticator: A,
//         capabilities: Arc<Mutex<Option<CapabilitiesMap>>>,
//         remote_authorized: bool,
//     ) -> Self {
//         Self {
//             inner,
//             control: control::Control::new(authenticator, capabilities, remote_authorized),
//         }
//     }

//     fn inner_if_unlocked(&self) -> Option<&S> {
//         self.control.remote_authorized().then_some(&self.inner)
//     }
// }

// impl<S> Service<S, PermissiveAuthenticator> {
//     fn client(inner: S, capabilities: Arc<Mutex<Option<CapabilitiesMap>>>) -> Self {
//         Self::new(inner, PermissiveAuthenticator, capabilities, true)
//     }
// }

// impl<S, A> tower::Service<(message::Address, BinaryValue)> for Service<S, A>
// where
//     S: tower::Service<(message::Address, BinaryValue)>,
//     A: Authenticator,
// {
//     type Response = S::Response;
//     type Error = S::Error;
//     type Future = ServiceCallFuture;

//     fn poll_ready(
//         &mut self,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Result<(), Self::Error>> {
//         todo!()
//     }

//     fn call(&self, (address, value): (message::Address, BinaryValue)) -> Self::Future {
//         let address = call.address();
//         if control::is_addressed_by(address) {
//             let authenticate = call
//                 .value_mut()
//                 .deserialize_value()
//                 .map(|request| self.control.authenticate(request));
//             async move { Ok(authenticate?.await?.serialize_value()?) }.boxed()
//         } else if let Some(inner) = self.inner_if_unlocked() {
//             inner.call(call)
//         } else {
//             ready(Err(NoMessageHandlerError(address).into())).boxed()
//         }
//     }
// }

// struct ServiceCallFuture;

// fn post(&self, post: messaging::Post) -> BoxFuture<'static, Result<(), messaging::Error>> {
//     if let Some(inner) = self.inner_if_unlocked() {
//         inner.post(post)
//     } else {
//         ready(Err(NoMessageHandlerError(post.address()).into())).boxed()
//     }
// }

// fn event(&self, event: messaging::Event) -> BoxFuture<'static, Result<(), messaging::Error>> {
//     if let Some(inner) = self.inner_if_unlocked() {
//         inner.event(event)
//     } else {
//         ready(Err(NoMessageHandlerError(event.address()).into())).boxed()
//     }
// }
