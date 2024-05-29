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
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
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

pub(super) async fn authenticate_to_remote<T, R>(
    client: &mut messaging::Client<T, R>,
    parameters: HashMap<String, Value<'_>>,
    capabilities: &watch::Sender<Option<CapabilitiesMap<'static>>>,
) -> Result<()>
where
    T: messaging::BodyBuf + Send,
    T::Error: Into<Error>,
    R: messaging::BodyBuf + Send,
    R::Error: Into<Error>,
{
    capabilities.send_replace(None); // Reset the capabilities
    let mut request = capabilities::local_map().clone();
    request.extend(
        parameters
            .into_iter()
            .map(|(k, v)| (k, Dynamic(v.into_owned()))),
    );
    let shared_capabilities = client
        .ready()
        .await?
        .call((
            AUTHENTICATE_ADDRESS,
            T::serialize(&request).map_err(Into::into)?,
        ))
        .await?
        .deserialize()
        .map_err(Into::into)?;
    capabilities::check_required(&shared_capabilities)?;
    capabilities.send_replace(Some(shared_capabilities));
    Ok(())
}

struct Control {
    authenticator: Box<dyn Authenticator>,
    capabilities: watch::Sender<Option<CapabilitiesMap<'static>>>,
    remote_authorized: AtomicBool,
}

impl Control {
    fn authenticate(&self, request: CapabilitiesMap<'_>) -> Result<CapabilitiesMap<'static>> {
        let shared_capabilities = capabilities::shared_with_local(&request);
        capabilities::check_required(&shared_capabilities)?;
        let parameters = request.into_iter().map(|(k, v)| (k, v.0)).collect();
        self.authenticator.verify(parameters)?;
        self.capabilities
            .send_replace(Some(shared_capabilities.clone()));
        self.remote_authorized
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(authentication::state_done_map(shared_capabilities))
    }

    fn is_remote_authorized(&self) -> bool {
        self.remote_authorized
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl std::fmt::Debug for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Control")
            .field("capabilities", &self.capabilities)
            .field("remote_authorized", &self.remote_authorized)
            .finish()
    }
}

pub(super) fn wrap<CallHandler, OnewaySink, In, Out>(
    call_handler: CallHandler,
    oneway_sink: OnewaySink,
    authenticator: Box<dyn Authenticator>,
    capabilities: watch::Sender<Option<CapabilitiesMap<'static>>>,
    remote_authorized: bool,
) -> (
    impl tower::Service<(message::Address, In), Response = Out, Error = Error>,
    impl Sink<(message::Address, message::OnewayRequest<In>), Error = Error>,
)
where
    CallHandler: tower::Service<(message::Address, In), Response = Out, Error = Error>,
    OnewaySink: Sink<(message::Address, message::OnewayRequest<In>), Error = Error>,
    In: messaging::BodyBuf<Error = Error>,
    Out: messaging::BodyBuf<Error = Error>,
{
    let control = Arc::new(Control {
        authenticator,
        capabilities,
        remote_authorized: AtomicBool::new(remote_authorized),
    });
    let controlled_call_handler = ControlledCallHandler {
        inner: call_handler,
        control: Arc::clone(&control),
    };
    let controlled_oneway_sink = oneway_sink.with_flat_map(move |request| {
        if control.is_remote_authorized() {
            stream::once(async move { Ok(request) }).left_stream()
        } else {
            stream::empty().right_stream()
        }
    });
    (controlled_call_handler, controlled_oneway_sink)
}

pub(super) struct ControlledCallHandler<H> {
    inner: H,
    control: Arc<Control>,
}

impl<S, In, Out> tower::Service<(message::Address, In)> for ControlledCallHandler<S>
where
    S: tower::Service<(message::Address, In), Response = Out, Error = Error>,
    In: messaging::BodyBuf<Error = Error>,
    Out: messaging::BodyBuf<Error = Error>,
{
    type Response = Out;
    type Error = Error;
    type Future = ControlledCallFuture<Out, S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, (address, value): (message::Address, In)) -> Self::Future {
        if is_addressed_by(address) {
            future::ready(
                value
                    .deserialize()
                    .and_then(|request| self.control.authenticate(request))
                    .and_then(|result| Out::serialize(&result)),
            )
            .left_future()
        } else if self.control.is_remote_authorized() {
            self.inner.call((address, value)).err_into().right_future()
        } else {
            future::err(Error::NoMessageHandler(address)).left_future()
        }
    }
}

type ControlledCallFuture<Out, F> =
    future::Either<future::Ready<Result<Out>>, future::ErrInto<F, Error>>;
