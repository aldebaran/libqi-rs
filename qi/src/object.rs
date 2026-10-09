pub use crate::value::ObjectId as Id;
use crate::{
    error::{FormatError, ValueConversionError},
    format,
    messaging::message,
    session::Session,
    value::{self, ActionId, Dynamic, FromValue, IntoValue, ServiceId, Value},
    Error,
};
use async_trait::async_trait;
use bytes::Bytes;
use sealed::sealed;
use tracing::{info, warn};
pub use value::object::{Uid, *};

// const ACTION_ID_REGISTER_EVENT: ActionId = ActionId(0);
// const ACTION_ID_UNREGISTER_EVENT: ActionId = ActionId(1);
const ACTION_ID_METAOBJECT: ActionId = ActionId(2);
// const ACTION_ID_TERMINATE: ActionId = ActionId(3);
pub const ACTION_ID_PROPERTY: ActionId = ActionId(5); // not a typo, there is no action 4
pub const ACTION_ID_SET_PROPERTY: ActionId = ActionId(6);
// const ACTION_ID_PROPERTIES: ActionId = ActionId(7);
// const ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE: ActionId = ActionId(8);
pub const ACTION_START_ID: ActionId = ActionId(100);

pub fn block_on<F>(future: F) -> F::Output
where
    F: std::future::Future,
{
    tokio::runtime::Handle::current().block_on(future)
}

pub(crate) struct BoxObject(Box<dyn Object + Send + Sync>);

impl BoxObject {
    pub(crate) fn new<T>(object: T) -> Self
    where
        T: Object + Send + Sync + 'static,
    {
        Self(Box::new(object))
    }
}

impl std::fmt::Debug for BoxObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BoxObject").field(self.0.meta()).finish()
    }
}

impl std::ops::Deref for BoxObject {
    type Target = dyn Object + Send + Sync;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T> From<T> for BoxObject
where
    T: Into<Box<dyn Object + Send + Sync>>,
{
    fn from(object: T) -> Self {
        Self(object.into())
    }
}

#[async_trait]
pub trait Object {
    fn meta(&self) -> &MetaObject;

    async fn meta_call(&self, ident: MemberIdent, args: Value<'_>)
        -> Result<Value<'static>, Error>;

    async fn meta_post(&self, ident: MemberIdent, value: Value<'_>);

    async fn meta_event(&self, ident: MemberIdent, value: Value<'_>);

    fn uid(&self) -> Uid {
        Uid::from_ptr(self)
    }
}

#[sealed]
#[async_trait]
pub trait ObjectExt: Object {
    async fn call<'a, R, Ident, T>(&self, ident: Ident, args: T) -> Result<R, Error>
    where
        Ident: Into<MemberIdent> + Send,
        T: IntoValue<'a> + Send,
        R: FromValue<'static>,
    {
        Ok(self
            .meta_call(ident.into(), args.into_value())
            .await?
            .cast_into()
            .map_err(ValueConversionError::MethodReturnValue)?)
    }

    async fn property<Ident, R>(&self, ident: Ident) -> Result<R, Error>
    where
        Ident: Into<MemberIdent> + Send,
        R: for<'r> FromValue<'r>,
    {
        self.call(ACTION_ID_PROPERTY, Dynamic(ident.into())).await
    }

    async fn set_property<Ident, T>(&self, ident: Ident, value: T) -> Result<(), Error>
    where
        Ident: Into<MemberIdent> + Send,
        T: for<'t> IntoValue<'t> + Send,
    {
        self.call(
            ACTION_ID_SET_PROPERTY,
            (Dynamic(ident.into()), Dynamic(value)),
        )
        .await
    }

    async fn properties(&self) -> Result<Vec<String>, Error> {
        Ok(self
            .meta()
            .properties
            .iter()
            .map(|(_uid, prop)| prop.name.clone())
            .collect())
    }
}

#[sealed]
#[async_trait]
impl<O> ObjectExt for O where O: Object + Sync + ?Sized {}

pub struct ObjectClient {
    service_id: ServiceId,
    id: Id,
    uid: Uid,
    meta: MetaObject,
    session: Session,
}

impl ObjectClient {
    pub(super) fn new(
        service_id: ServiceId,
        id: Id,
        uid: Uid,
        meta: MetaObject,
        session: Session,
    ) -> Self {
        Self {
            service_id,
            id,
            uid,
            meta,
            session,
        }
    }

    pub(super) async fn connect(
        service_id: ServiceId,
        id: Id,
        uid: Uid,
        session: Session,
    ) -> Result<Self, Error> {
        let meta = Self::fetch_meta_object(&session, service_id, id).await?;
        Ok(Self {
            service_id,
            id,
            uid,
            meta,
            session,
        })
    }

    async fn fetch_meta_object(
        session: &Session,
        service_id: ServiceId,
        id: Id,
    ) -> Result<MetaObject, Error> {
        Ok(session
            .call(
                message::Address(service_id, id, ACTION_ID_METAOBJECT),
                0.into_value(), // unused
                <MetaObject as value::Reflect>::signature()
                    .into_type()
                    .as_ref(),
            )
            .await?
            .cast_into()
            .map_err(ValueConversionError::MethodReturnValue)?)
    }

    fn builtin_return_type(
        &self,
        action: ActionId,
        args: &Value<'_>,
    ) -> Result<Option<value::Type>, Error> {
        if action == ACTION_ID_SET_PROPERTY {
            return Ok(<() as value::Reflect>::signature().into_type());
        }
        let prop_ident: Dynamic<MemberIdent> = args
            .clone()
            .cast_into()
            .map_err(|err| Error::from(crate::BoxError::from(err)))?;
        let property = self
            .meta
            .property(&prop_ident.0)
            .ok_or_else(|| Error::MethodNotFound(prop_ident.0))?;
        Ok(property.signature.clone().into_type())
    }
}

impl Clone for ObjectClient {
    fn clone(&self) -> Self {
        Self {
            service_id: self.service_id,
            id: self.id,
            uid: self.uid,
            meta: self.meta.clone(),
            session: self.session.clone(),
        }
    }
}

impl std::fmt::Debug for ObjectClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectClient")
            .field("service_id", &self.service_id)
            .field("id", &self.id)
            .field("uid", &self.uid)
            .field("meta", &self.meta)
            .field("session", &self.session)
            .finish()
    }
}

#[async_trait]
impl Object for ObjectClient {
    fn meta(&self) -> &MetaObject {
        &self.meta
    }

    async fn meta_call(
        &self,
        ident: MemberIdent,
        args: Value<'_>,
    ) -> Result<Value<'static>, Error> {
        if let MemberIdent::Id(action @ (ACTION_ID_PROPERTY | ACTION_ID_SET_PROPERTY)) = ident {
            let return_type = self.builtin_return_type(action, &args)?;
            return self
                .session
                .call(
                    message::Address(self.service_id, self.id, action),
                    args,
                    return_type.as_ref(),
                )
                .await;
        }
        let method = self
            .meta
            .method(&ident)
            .ok_or_else(|| Error::MethodNotFound(ident))?;
        self.session
            .call(
                message::Address(self.service_id, self.id, method.uid),
                args,
                method.return_signature.to_type(),
            )
            .await
    }

    async fn meta_post(&self, ident: MemberIdent, args: Value<'_>) {
        let target = match PostTarget::get(&self.meta, &ident) {
            Some(target) => target,
            None => {
                warn!(
                    member = %ident,
                    "post request error: target not found"
                );
                return;
            }
        };
        if let Err(err) = self
            .session
            .post(
                message::Address(self.service_id, self.id, target.action_id()),
                args,
            )
            .await
        {
            warn!(
                error = &err as &dyn std::error::Error,
                "post request error: failure to send"
            );
        }
    }

    async fn meta_event(&self, ident: MemberIdent, value: Value<'_>) {
        let signal = match self.meta.signal(&ident) {
            Some(signal) => signal,
            None => {
                warn!(
                    member = %ident,
                    "event request error: signal not found"
                );
                return;
            }
        };
        if let Err(err) = self
            .session
            .send_event(
                message::Address(self.service_id, self.id, signal.uid),
                value,
            )
            .await
        {
            warn!(
                error = &err as &dyn std::error::Error,
                "event request error: failure to send"
            );
        }
    }

    fn uid(&self) -> Uid {
        self.uid
    }
}

#[derive(Debug)]
enum PostTarget<'a> {
    Method(&'a MetaMethod),
    Signal(&'a MetaSignal),
}

impl<'a> PostTarget<'a> {
    fn get(meta: &'a MetaObject, ident: &MemberIdent) -> Option<Self> {
        meta.method(ident)
            .map(Self::Method)
            .or_else(|| meta.signal(ident).map(Self::Signal))
    }

    fn action_id(&self) -> ActionId {
        match self {
            PostTarget::Method(method) => method.uid,
            PostTarget::Signal(signal) => signal.uid,
        }
    }

    fn parameters_signature(&self) -> &value::Signature {
        match self {
            PostTarget::Method(method) => &method.parameters_signature,
            PostTarget::Signal(signal) => &signal.signature,
        }
    }
}

/// An messaging handler-like interface for object, but not exactly one. Messaging handlers take
/// messaging address as parameter, while this interface only takes action identifiers (so without the
/// service and object identifiers in messaging addresses).
#[async_trait]
pub(super) trait HandlerExt: Object {
    async fn handler_meta_call(&self, action: ActionId, args: Bytes) -> Result<Bytes, Error> {
        // Get the targeted method so that we can get the expected parameters type and know what
        // type of value we're supposed to deserialize.
        if action == ACTION_ID_METAOBJECT {
            let reply = self.meta().clone().into_value();
            return Ok(
                format::to_bytes(&reply).map_err(FormatError::MethodReturnValueSerialization)?
            );
        }
        let action_ident = MemberIdent::Id(action);
        let method = self
            .meta()
            .method(&action_ident)
            .ok_or_else(|| Error::MethodNotFound(action_ident.clone()))?;
        let args = value::deserialize(method.parameters_signature.to_type(), &args)
            .map_err(FormatError::ArgumentsDeserialization)?;
        let reply = self.meta_call(action_ident, call_arguments(args)).await?;
        Ok(format::to_bytes(&reply).map_err(FormatError::MethodReturnValueSerialization)?)
    }

    async fn handler_meta_post(&self, action: ActionId, args: Bytes) {
        // Same as for "call", we need to know the type of parameters to know what to deserialize.
        let action_ident = MemberIdent::Id(action);
        let target = match PostTarget::get(self.meta(), &action_ident) {
            Some(target) => target,
            None => {
                info!(
                    target = %action_ident,
                    "post request discarded: action target not found"
                );
                return;
            }
        };
        match value::deserialize(target.parameters_signature().to_type(), &args) {
            Ok(args) => self.meta_post(action_ident, args).await,
            Err(err) => info!(
                error = &err as &dyn std::error::Error,
                "post request discarded: failed to deserialize arguments"
            ),
        };
    }

    async fn handler_meta_event(&self, action: ActionId, args: Bytes) {
        let action_ident = MemberIdent::Id(action);
        let signal = match self.meta().signal(&action_ident) {
            Some(signal) => signal,
            None => {
                info!(
                    signal = %action_ident,
                    "event request discarded: signal not found"
                );
                return;
            }
        };
        match value::deserialize(signal.signature.to_type(), &args) {
            Ok(args) => self.meta_event(action_ident, args).await,
            Err(err) => info!(
                error = &err as &dyn std::error::Error,
                "event request discarded: failed to deserialize arguments"
            ),
        };
    }
}

impl<O> HandlerExt for O where O: Object + Sync + ?Sized {}

/// Wire parameters are a tuple. Dispatch matches a local call: `()` , one value, or the tuple.
fn call_arguments(value: Value<'_>) -> Value<'_> {
    match value {
        Value::Tuple(mut elements) if elements.len() == 1 => elements.pop().unwrap(),
        Value::Tuple(elements) if elements.is_empty() => {
            let _ = elements;
            Value::Unit
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Reflect;
    use assert_matches::assert_matches;
    use async_trait::async_trait;
    use futures::FutureExt;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[allow(dead_code)]
    #[qi::object]
    trait Calculator {
        /// Go to some position.
        #[qi::method(name = "goTo")]
        async fn go_to(&self, position: i32) -> Result<(), Error>;

        /// The current position.
        #[qi::property]
        fn position(&self) -> crate::Property<i32>;

        /// The moving state.
        #[qi::signal]
        fn moving(&self) -> crate::Signal<bool>;

        #[qi::method]
        async fn add(&self, b: i32) -> i32;

        #[qi::method]
        async fn sub(&self, b: i32) -> i32;

        #[qi::method]
        async fn mul(&self, b: i32) -> i32;

        #[qi::method]
        async fn div(&self, b: i32) -> std::result::Result<i32, DivisionByZeroError>;

        #[qi::method]
        async fn clamp(&self, min: i32, max: i32) -> i32;

        #[qi::method]
        async fn ans(&self) -> i32;
    }

    struct Calc {
        a: Mutex<i32>,
        position: crate::Property<i32>,
        moving: crate::Signal<bool>,
    }

    impl Calc {
        fn new(a: i32) -> Self {
            Self {
                a: Mutex::new(a),
                position: crate::Property::new(0),
                moving: crate::Signal::new(),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("division by zero")]
    struct DivisionByZeroError;

    impl From<DivisionByZeroError> for Error {
        fn from(err: DivisionByZeroError) -> Self {
            Self::Other(err.into())
        }
    }

    impl From<Error> for DivisionByZeroError {
        fn from(_err: Error) -> Self {
            Self
        }
    }

    #[async_trait]
    impl Calculator for Calc {
        async fn go_to(&self, _position: i32) -> Result<(), Error> {
            Ok(())
        }

        fn position(&self) -> crate::Property<i32> {
            self.position.clone()
        }

        fn moving(&self) -> crate::Signal<bool> {
            self.moving.clone()
        }

        async fn add(&self, b: i32) -> i32 {
            let mut a = self.a.lock().await;
            *a += b;
            *a
        }

        async fn sub(&self, b: i32) -> i32 {
            let mut a = self.a.lock().await;
            *a -= b;
            *a
        }

        async fn mul(&self, b: i32) -> i32 {
            let mut a = self.a.lock().await;
            *a *= b;
            *a
        }

        async fn div(&self, b: i32) -> std::result::Result<i32, DivisionByZeroError> {
            let mut a = self.a.lock().await;
            if b == 0 {
                Err(DivisionByZeroError)
            } else {
                *a /= b;
                Ok(*a)
            }
        }

        async fn clamp(&self, min: i32, max: i32) -> i32 {
            let mut a = self.a.lock().await;
            *a = (*a).clamp(min, max);
            *a
        }

        async fn ans(&self) -> i32 {
            *self.a.lock().await
        }
    }

    #[tokio::test]
    async fn test_calculator_object_call_methods() {
        let calc = Calc::new(42);
        let res: i32 = calc.call("add", 100).await.unwrap();
        assert_eq!(res, 142);
        let res: i32 = calc.call("add", 50).await.unwrap();
        assert_eq!(res, 192);
        let res: i32 = calc.call("sub", 12).await.unwrap();
        assert_eq!(res, 180);
        let res: i32 = calc.call("div", 90).await.unwrap();
        assert_eq!(res, 2);
        let res: i32 = calc.call("mul", 64).await.unwrap();
        assert_eq!(res, 128);
        let res: i32 = calc.call("clamp", (32, 127)).await.unwrap();
        assert_eq!(res, 127);
        let res: Result<i32, _> = calc.call("div", 0).await;
        assert_matches!(res, Err(Error::Other(err)) => {
            assert!(err.downcast::<DivisionByZeroError>().is_ok())
        });
        let res: Result<i32, _> = calc.call("log", 1).await;
        assert_matches!(
            res,
            Err(Error::MethodNotFound(ident)) => assert_eq!(ident, "log")
        );
        let res: i32 = calc.call("ans", ()).await.unwrap();
        assert_eq!(res, 127);
    }

    #[tokio::test]
    async fn property_set_emits_and_signal_event_reaches_subscriber() {
        let calc = Calc::new(0);
        let property_updates = std::sync::Arc::new(std::sync::Mutex::new(None));
        let property_slot = std::sync::Arc::clone(&property_updates);
        calc.position().subscribe(move |value| {
            *property_slot.lock().unwrap() = Some(value);
        });
        let signal_updates = std::sync::Arc::new(std::sync::Mutex::new(None));
        let signal_slot = std::sync::Arc::clone(&signal_updates);
        calc.moving().subscribe(move |value| {
            *signal_slot.lock().unwrap() = Some(value);
        });

        let position: i32 = calc.property("position").await.unwrap();
        assert_eq!(position, 0);
        calc.set_property("position", 4).await.unwrap();
        let position: i32 = calc.property("position").await.unwrap();
        assert_eq!(position, 4);
        assert_eq!(*property_updates.lock().unwrap(), Some(4));

        calc.meta_event(MemberIdent::from("moving"), true.into_value())
            .await;
        assert_eq!(*signal_updates.lock().unwrap(), Some(true));
    }

    #[test]
    fn emits_meta_object() {
        let meta = &*CALCULATOR_META_OBJECT;

        let go_to = meta
            .method(&MemberIdent::from("goTo"))
            .expect("method goTo");
        assert_eq!(go_to.uid, ACTION_START_ID);
        assert_eq!(go_to.name, "goTo");
        assert_eq!(go_to.description, "Go to some position.");
        assert_eq!(go_to.parameters[0].name, "position");
        assert_eq!(go_to.parameters_signature, <(i32,) as Reflect>::signature());
        assert_eq!(go_to.return_signature, <() as Reflect>::signature());

        let position = meta
            .property(&MemberIdent::from("position"))
            .expect("property position");
        assert_eq!(position.uid, ActionId(101));
        assert_eq!(position.signature, <i32 as Reflect>::signature());

        let moving = meta
            .signal(&MemberIdent::from("moving"))
            .expect("signal moving");
        assert_eq!(moving.uid, ActionId(102));
        assert_eq!(moving.signature, <bool as Reflect>::signature());
    }

    #[tokio::test]
    async fn client_rejects_meta_object_missing_go_to() {
        let (_server, session) = host(Arc::new(Calc::new(0))).await;
        let mut meta = CALCULATOR_META_OBJECT.clone();
        let go_to = meta
            .method(&MemberIdent::from("goTo"))
            .expect("method goTo")
            .uid;
        meta.methods.retain(|id, _| *id != go_to);
        let client = ObjectClient::new(ServiceId(2), Id(1), Uid::default(), meta, session);
        let err = CalculatorClient::try_from(client).unwrap_err();
        assert_matches!(err, Error::MethodNotFound(ident) => assert_eq!(ident, "goTo"));
    }

    #[tokio::test]
    async fn client_calls_method_over_localhost() {
        let calc = Arc::new(Calc::new(42));
        let (_server, session) = host(Arc::clone(&calc)).await;
        let client = ObjectClient::connect(ServiceId(2), Id(1), Uid::default(), session)
            .await
            .unwrap();
        let client = CalculatorClient::try_from(client).unwrap();
        let sum: i32 = client.add(100).await;
        assert_eq!(sum, 142);
    }

    async fn host(calc: Arc<Calc>) -> (crate::session::Server, crate::session::Session) {
        let mut server = crate::session::Session::server(
            "tcp://127.0.0.1:0".parse().unwrap(),
            crate::session::auth::PermissiveAuthenticator,
            Host(calc),
        )
        .await
        .unwrap();
        let address = server.endpoints_receiver().borrow().0;
        let (incoming, outgoing) = crate::messaging::channel::connect(address).await.unwrap();
        let session =
            crate::session::Session::connect(incoming, outgoing, Default::default(), Idle)
                .await
                .unwrap();
        (server, session)
    }

    #[derive(Clone)]
    struct Host(Arc<Calc>);

    impl crate::messaging::CallHandler for Host {
        type Error = crate::HandlerError;
        type Future = futures::future::BoxFuture<'static, Result<bytes::Bytes, Self::Error>>;

        fn handle_call(
            &self,
            address: crate::messaging::message::Address,
            args: bytes::Bytes,
        ) -> Self::Future {
            let object = Arc::clone(&self.0);
            async move {
                if address.service() != ServiceId(2) || address.object() != Id(1) {
                    return Err(crate::HandlerError::non_fatal("no handler"));
                }
                object
                    .handler_meta_call(address.action(), args)
                    .await
                    .map_err(crate::HandlerError::non_fatal)
            }
            .boxed()
        }
    }

    impl crate::messaging::EventHandler for Host {
        fn handle_event(&self, _address: crate::messaging::message::Address, _args: bytes::Bytes) {}
    }

    impl crate::messaging::PostHandler for Host {
        fn handle_post(&self, _address: crate::messaging::message::Address, _args: bytes::Bytes) {}
    }

    impl crate::messaging::CapabilitiesHandler for Host {
        fn handle_capabilities(
            &self,
            _address: crate::messaging::message::Address,
            _data: crate::value::KeyDynValueMap,
        ) {
        }
    }

    #[derive(Clone, Copy)]
    struct Idle;

    impl crate::messaging::CallHandler for Idle {
        type Error = crate::HandlerError;
        type Future = futures::future::Ready<Result<bytes::Bytes, Self::Error>>;

        fn handle_call(
            &self,
            _address: crate::messaging::message::Address,
            _args: bytes::Bytes,
        ) -> Self::Future {
            futures::future::ready(Err(crate::HandlerError::non_fatal("no handler")))
        }
    }

    impl crate::messaging::EventHandler for Idle {
        fn handle_event(&self, _address: crate::messaging::message::Address, _args: bytes::Bytes) {}
    }

    impl crate::messaging::PostHandler for Idle {
        fn handle_post(&self, _address: crate::messaging::message::Address, _args: bytes::Bytes) {}
    }

    impl crate::messaging::CapabilitiesHandler for Idle {
        fn handle_capabilities(
            &self,
            _address: crate::messaging::message::Address,
            _data: crate::value::KeyDynValueMap,
        ) {
        }
    }
}
