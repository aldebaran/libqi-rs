pub use crate::value::ObjectId as Id;
use crate::{
    error::{FormatError, ValueConversionError},
    messaging::{self, message},
    session::Session,
    value::{self, ActionId, Dynamic, FromValue, IntoValue, ServiceId, Value},
    Error,
};
use async_trait::async_trait;
use sealed::sealed;
use tracing::{info, warn};
pub use value::object::{Uid, *};

// const ACTION_ID_REGISTER_EVENT: ActionId = ActionId(0);
// const ACTION_ID_UNREGISTER_EVENT: ActionId = ActionId(1);
const ACTION_ID_METAOBJECT: ActionId = ActionId(2);
// const ACTION_ID_TERMINATE: ActionId = ActionId(3);
const ACTION_ID_PROPERTY: ActionId = ActionId(5); // not a typo, there is no action 4
const ACTION_ID_SET_PROPERTY: ActionId = ActionId(6);
// const ACTION_ID_PROPERTIES: ActionId = ActionId(7);
// const ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE: ActionId = ActionId(8);
pub const ACTION_START_ID: ActionId = ActionId(100);

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

pub struct Proxy<Body> {
    service_id: ServiceId,
    id: Id,
    uid: Uid,
    meta: MetaObject,
    session: Session<Body>,
}

impl<Body> Proxy<Body> {
    pub(super) fn new(
        service_id: ServiceId,
        id: Id,
        uid: Uid,
        meta: MetaObject,
        session: Session<Body>,
    ) -> Self {
        Self {
            service_id,
            id,
            uid,
            meta,
            session,
        }
    }
}

impl<Body> Proxy<Body>
where
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    pub(super) async fn connect(
        service_id: ServiceId,
        id: Id,
        uid: Uid,
        session: Session<Body>,
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
        session: &Session<Body>,
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
}

impl<Body> Clone for Proxy<Body> {
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

impl<Body> std::fmt::Debug for Proxy<Body> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Proxy")
            .field("service_id", &self.service_id)
            .field("id", &self.id)
            .field("uid", &self.uid)
            .field("meta", &self.meta)
            .field("session", &self.session)
            .finish()
    }
}

#[async_trait]
impl<Body> Object for Proxy<Body>
where
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    fn meta(&self) -> &MetaObject {
        &self.meta
    }

    async fn meta_call(
        &self,
        ident: MemberIdent,
        args: Value<'_>,
    ) -> Result<Value<'static>, Error> {
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
            .fire_and_forget(
                message::Address(self.service_id, self.id, target.action_id()),
                message::FireAndForget::Post(args),
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
            .fire_and_forget(
                message::Address(self.service_id, self.id, signal.uid),
                message::FireAndForget::Event(value),
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
pub(super) trait HandlerExt<Body>: Object
where
    Body: messaging::Body + Send,
    Body::Error: std::error::Error + Send + Sync + 'static,
{
    async fn handler_meta_call<'a>(&'a self, action: ActionId, args: Body) -> Result<Body, Error>
    where
        Body: 'a,
    {
        // Get the targeted method so that we can get the expected parameters type and know what
        // type of value we're supposed to deserialize.
        let action_ident = MemberIdent::Id(action);
        let method = self
            .meta()
            .method(&action_ident)
            .ok_or_else(|| Error::MethodNotFound(action_ident.clone()))?;
        let args = args
            .deserialize_seed(value::de::ValueType(method.parameters_signature.to_type()))
            .map_err(FormatError::ArgumentsDeserialization)?;
        let reply = self.meta_call(action_ident, args).await?;
        Ok(Body::serialize(&reply).map_err(FormatError::MethodReturnValueSerialization)?)
    }

    async fn handler_meta_post<'a>(&'a self, action: ActionId, args: Body)
    where
        Body: 'a,
    {
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
        match args.deserialize_seed(value::de::ValueType(
            target.parameters_signature().to_type(),
        )) {
            Ok(args) => self.meta_post(action_ident, args).await,
            Err(err) => info!(
                error = &err as &dyn std::error::Error,
                "post request discarded: failed to deserialize arguments"
            ),
        };
    }

    async fn handler_meta_event<'a>(&'a self, action: ActionId, args: Body)
    where
        Body: 'a,
    {
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
        match args.deserialize_seed(value::de::ValueType(signal.signature.to_type())) {
            Ok(args) => self.meta_event(action_ident, args).await,
            Err(err) => info!(
                error = &err as &dyn std::error::Error,
                "event request discarded: failed to deserialize arguments"
            ),
        };
    }
}

impl<Body, O> HandlerExt<Body> for O
where
    O: Object + Sync + ?Sized,
    Body: messaging::Body + Send,
    Body::Error: std::error::Error + Send + Sync + 'static,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use async_trait::async_trait;
    use once_cell::sync::Lazy;
    use qi_value::{
        object::{MetaMethod, MetaObject},
        ActionId, Type, Value,
    };
    use tokio::sync::Mutex;

    #[derive(Debug)]
    struct Calculator {
        a: i32,
    }

    impl Calculator {
        fn new(a: i32) -> Self {
            Self { a }
        }

        fn add(&mut self, b: i32) -> i32 {
            self.a += b;
            self.a
        }

        fn sub(&mut self, b: i32) -> i32 {
            self.a -= b;
            self.a
        }

        fn mul(&mut self, b: i32) -> i32 {
            self.a *= b;
            self.a
        }

        fn div(&mut self, b: i32) -> std::result::Result<i32, DivisionByZeroError> {
            if b == 0 {
                Err(DivisionByZeroError)
            } else {
                self.a /= b;
                Ok(self.a)
            }
        }

        fn clamp(&mut self, min: i32, max: i32) -> i32 {
            self.a = self.a.clamp(min, max);
            self.a
        }

        fn ans(&self) -> i32 {
            self.a
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("division by zero")]
    struct DivisionByZeroError;

    #[derive(Debug)]
    struct Meta {
        object: MetaObject,
        methods: MethodIds,
    }

    impl Meta {
        fn get() -> &'static Self {
            static META: Lazy<Meta> = Lazy::new(|| {
                let mut method_id = ActionId(0);
                let mut builder = MetaObject::builder();
                let add;
                let sub;
                let mul;
                let div;
                let clamp;
                let ans;
                builder
                    .add_method({
                        add = method_id.wrapping_next();
                        let mut builder = MetaMethod::builder(add);
                        builder.set_name("add");
                        builder.parameter(0).set_type(Type::Int32);
                        builder.return_value().set_type(Type::Int32);
                        builder.build()
                    })
                    .add_method({
                        sub = method_id.wrapping_next();
                        let mut builder = MetaMethod::builder(sub);
                        builder.set_name("sub");
                        builder.parameter(0).set_type(Type::Int32);
                        builder.build()
                    })
                    .add_method({
                        mul = method_id.wrapping_next();
                        let mut builder = MetaMethod::builder(mul);
                        builder.set_name("mul");
                        builder.parameter(0).set_type(Type::Int32);
                        builder.build()
                    })
                    .add_method({
                        div = method_id.wrapping_next();
                        let mut builder = MetaMethod::builder(div);
                        builder.set_name("div");
                        builder.parameter(0).set_type(Type::Int32);
                        builder.build()
                    })
                    .add_method({
                        clamp = method_id.wrapping_next();
                        let mut builder = MetaMethod::builder(clamp);
                        builder.set_name("clamp");
                        builder.parameter(0).set_type(Type::Int32);
                        builder.parameter(1).set_type(Type::Int32);
                        builder.build()
                    })
                    .add_method({
                        ans = method_id.wrapping_next();
                        let mut builder = MetaMethod::builder(ans);
                        builder.set_name("ans");
                        builder.build()
                    });
                let object = builder.build();
                let methods = MethodIds {
                    add,
                    sub,
                    mul,
                    div,
                    clamp,
                    ans,
                };
                Meta { object, methods }
            });
            &META
        }
    }

    #[derive(Debug)]
    struct MethodIds {
        add: ActionId,
        sub: ActionId,
        mul: ActionId,
        div: ActionId,
        clamp: ActionId,
        ans: ActionId,
    }

    #[derive(Debug)]
    enum Method {
        Add,
        Sub,
        Mul,
        Div,
        Clamp,
        Ans,
    }

    impl Method {
        fn from_ident(ident: &MemberIdent) -> Option<Self> {
            let Meta { object, methods } = Meta::get();
            object.method(ident).and_then(|method| {
                let id = method.uid;
                if id == methods.add {
                    Some(Method::Add)
                } else if id == methods.sub {
                    Some(Method::Sub)
                } else if id == methods.mul {
                    Some(Method::Mul)
                } else if id == methods.div {
                    Some(Method::Div)
                } else if id == methods.clamp {
                    Some(Method::Clamp)
                } else if id == methods.ans {
                    Some(Method::Ans)
                } else {
                    None
                }
            })
        }

        fn call(self, calc: &mut Calculator, args: Value<'_>) -> Result<Value<'static>, Error> {
            Ok(match &self {
                Self::Add => {
                    let arg = args.cast_into().map_err(ValueConversionError::Arguments)?;
                    calc.add(arg).into_value()
                }
                Self::Sub => {
                    let arg = args.cast_into().map_err(ValueConversionError::Arguments)?;
                    calc.sub(arg).into_value()
                }
                Self::Mul => {
                    let arg = args.cast_into().map_err(ValueConversionError::Arguments)?;
                    calc.mul(arg).into_value()
                }
                Self::Div => {
                    let arg = args.cast_into().map_err(ValueConversionError::Arguments)?;
                    calc.div(arg)
                        .map_err(Into::into)
                        .map_err(Error::Other)?
                        .into_value()
                }
                Self::Clamp => {
                    let (arg1, arg2) = args.cast_into().map_err(ValueConversionError::Arguments)?;
                    calc.clamp(arg1, arg2).into_value()
                }
                Self::Ans => {
                    let () = args.cast_into().map_err(ValueConversionError::Arguments)?;
                    calc.ans().into_value()
                }
            })
        }
    }

    #[async_trait]
    impl Object for Mutex<Calculator> {
        fn meta(&self) -> &MetaObject {
            &Meta::get().object
        }

        async fn meta_call(
            &self,
            ident: MemberIdent,
            args: Value<'_>,
        ) -> Result<Value<'static>, Error> {
            Method::from_ident(&ident)
                .ok_or_else(|| Error::MethodNotFound(ident))?
                .call(&mut *self.lock().await, args)
        }

        async fn meta_post(&self, ident: MemberIdent, args: Value<'_>) {
            let _res = self.meta_call(ident, args).await;
        }

        async fn meta_event(&self, _ident: MemberIdent, _value: Value<'_>) {
            // no signal
        }
    }

    #[tokio::test]
    async fn test_calculator_object_call_methods() {
        let calc = Mutex::new(Calculator::new(42));
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
}
