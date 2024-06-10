use super::{
    authentication::{self, Authenticator},
    capabilities,
};
use crate::{
    messaging::{self, CapabilitiesMap},
    Error, Result,
};
use futures::{future, stream, FutureExt, Sink, SinkExt, StreamExt, TryFutureExt};
use messaging::message;
use qi_value::{ActionId, Dynamic, ObjectId, ServiceId, Value};
use serde::Deserialize;
use std::{
    collections::HashMap,
    future::Future,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::watch;
use tower::{Service, ServiceExt};

const SERVICE_ID: ServiceId = ServiceId(0);
const OBJECT_ID: ObjectId = ObjectId(0);
const AUTHENTICATE_ACTION_ID: ActionId = ActionId(8);

fn is_addressed_by(address: message::Address) -> bool {
    address.service() == SERVICE_ID && address.object() == OBJECT_ID
}

const AUTHENTICATE_ADDRESS: message::Address =
    message::Address(SERVICE_ID, OBJECT_ID, AUTHENTICATE_ACTION_ID);

pub(super) struct Controller {
    authenticator: Box<dyn Authenticator + Send + Sync>,
    capabilities: watch::Sender<Option<CapabilitiesMap<'static>>>,
    remote_authorized: watch::Sender<bool>,
}

impl Controller {
    fn authenticate(&self, request: CapabilitiesMap<'_>) -> Result<CapabilitiesMap<'static>> {
        let shared_capabilities = capabilities::shared_with_local(&request);
        capabilities::check_required(&shared_capabilities)?;
        let parameters = request.into_iter().map(|(k, v)| (k, v.0)).collect();
        self.authenticator.verify(parameters)?;
        self.capabilities
            .send_replace(Some(shared_capabilities.clone()));
        self.remote_authorized.send_replace(true);
        Ok(authentication::state_done_map(shared_capabilities))
    }

    pub(super) async fn authenticate_to_remote<T, R>(
        &self,
        client: &mut messaging::Client<T, R>,
        parameters: HashMap<String, Value<'_>>,
    ) -> Result<()>
    where
        T: messaging::BodyBuf + Send,
        T::Error: Into<Error>,
        R: messaging::BodyBuf + Send,
        R::Error: Into<Error>,
        for<'de> <R::Deserializer<'de> as serde::Deserializer<'de>>::Error: Into<R::Error>,
    {
        self.capabilities.send_replace(None); // Reset the capabilities
        let mut request = capabilities::local_map().clone();
        request.extend(
            parameters
                .into_iter()
                .map(|(k, v)| (k, Dynamic(v.into_owned()))),
        );
        let mut authenticate_result = client
            .ready()
            .await?
            .call((
                AUTHENTICATE_ADDRESS,
                T::serialize(&request).map_err(Into::into)?,
            ))
            .await?;
        let mut shared_capabilities = Deserialize::deserialize(authenticate_result.deserializer())
            .map_err(Into::into)
            .map_err(Into::into)?;
        authentication::extract_state_result(&mut shared_capabilities)?;
        capabilities::check_required(&shared_capabilities)?;
        self.capabilities.send_replace(Some(shared_capabilities));
        Ok(())
    }
}

impl std::fmt::Debug for Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Control")
            .field("capabilities", &self.capabilities)
            .field("remote_authorized", &self.remote_authorized)
            .finish()
    }
}

pub(super) struct Control<H, S> {
    pub(super) controller: Arc<Controller>,
    pub(super) capabilities: watch::Receiver<Option<CapabilitiesMap<'static>>>,
    pub(super) remote_authorized: watch::Receiver<bool>,
    pub(super) call_handler: ControlledCallHandler<H>,
    pub(super) oneway_sink: S,
}

pub(super) fn make<CallHandler, OnewaySink, Auth, In, Out>(
    call_handler: CallHandler,
    oneway_sink: OnewaySink,
    authenticator: Auth,
    remote_authorized: bool,
) -> Control<CallHandler, impl Sink<(message::Address, message::OnewayRequest<In>), Error = Error>>
where
    CallHandler: tower::Service<(message::Address, In), Response = Out, Error = Error>,
    OnewaySink: Sink<(message::Address, message::OnewayRequest<In>), Error = Error>,
    Auth: Authenticator + Send + Sync + 'static,
    In: messaging::BodyBuf<Error = Error>,
    Out: messaging::BodyBuf<Error = Error>,
{
    let (capabilities_sender, capabilities_receiver) = watch::channel(Default::default());
    let (remote_authorized_sender, remote_authorized_receiver) = watch::channel(remote_authorized);
    let controller = Arc::new(Controller {
        authenticator: Box::new(authenticator),
        capabilities: capabilities_sender,
        remote_authorized: remote_authorized_sender,
    });
    let controlled_call_handler = ControlledCallHandler {
        inner: call_handler,
        controller: Arc::clone(&controller),
    };
    let controlled_oneway_sink = oneway_sink.with_flat_map({
        let controller = Arc::clone(&controller);
        move |request| {
            if *controller.remote_authorized.borrow() {
                stream::once(async move { Ok(request) }).left_stream()
            } else {
                stream::empty().right_stream()
            }
        }
    });
    Control {
        controller,
        capabilities: capabilities_receiver,
        remote_authorized: remote_authorized_receiver,
        call_handler: controlled_call_handler,
        oneway_sink: controlled_oneway_sink,
    }
}

pub(super) struct ControlledCallHandler<H> {
    inner: H,
    controller: Arc<Controller>,
}

impl<H> ControlledCallHandler<H> {
    pub(super) fn remote_authorized(&self) -> impl Future<Output = ()> {
        let mut remote_authorized = self.controller.remote_authorized.subscribe();
        async move {
            remote_authorized.wait_for(|authorized| *authorized).await;
        }
    }
}

impl<S, In, Out> tower::Service<(message::Address, In)> for ControlledCallHandler<S>
where
    S: tower::Service<(message::Address, In), Response = Out, Error = Error>,
    In: messaging::BodyBuf<Error = Error>,
    for<'de> <In::Deserializer<'de> as serde::Deserializer<'de>>::Error: Into<In::Error>,
    Out: messaging::BodyBuf<Error = Error>,
{
    type Response = Out;
    type Error = Error;
    type Future = ControlledCallFuture<Out, S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, (address, mut value): (message::Address, In)) -> Self::Future {
        if is_addressed_by(address) {
            let controller = Arc::clone(&self.controller);
            future::ready(
                Deserialize::deserialize(value.deserializer())
                    .map_err(Into::into)
                    .and_then(|request| controller.authenticate(request))
                    .and_then(|result| Out::serialize(&result)),
            )
            .left_future()
        } else if *self.controller.remote_authorized.borrow() {
            self.inner.call((address, value)).err_into().right_future()
        } else {
            future::err(Error::NoMessageHandler(address)).left_future()
        }
    }
}

type ControlledCallFuture<Out, F> =
    future::Either<future::Ready<Result<Out>>, future::ErrInto<F, Error>>;
