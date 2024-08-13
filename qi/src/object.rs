pub use crate::value::ObjectId as Id;
use crate::{
    messaging::message,
    os,
    session::Session,
    value::{
        self, ActionId, Dynamic, FromValue, IntoValue, Reflect, RuntimeReflect, ServiceId, Value,
    },
};
use async_trait::async_trait;
use sealed::sealed;
use sha1::{Digest, Sha1};
use std::borrow::Cow;
pub use value::object::*;

// const ACTION_ID_REGISTER_EVENT: ActionId = ActionId(0);
// const ACTION_ID_UNREGISTER_EVENT: ActionId = ActionId(1);
const ACTION_ID_METAOBJECT: ActionId = ActionId(2);
// const ACTION_ID_TERMINATE: ActionId = ActionId(3);
const ACTION_ID_PROPERTY: ActionId = ActionId(5); // not a typo, there is no action 4
const ACTION_ID_SET_PROPERTY: ActionId = ActionId(6);
// const ACTION_ID_PROPERTIES: ActionId = ActionId(7);
// const ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE: ActionId = ActionId(8);
pub const ACTION_START_ID: ActionId = ActionId(100);

#[async_trait]
pub trait Object {
    fn meta(&self) -> &MetaObject;

    async fn meta_call(&self, ident: MemberIdent, args: Value<'_>)
        -> Result<Value<'static>, Error>;

    async fn meta_post(&self, ident: MemberIdent, value: Value<'_>) -> Result<(), Error>;

    async fn meta_event(&self, ident: MemberIdent, value: Value<'_>) -> Result<(), Error>;

    fn uid(&self) -> Uid {
        Uid::from_ptr(self)
    }
}

pub type BoxObject = Box<dyn Object + Send + Sync>;

#[sealed]
#[async_trait]
pub trait ObjectExt: Object {
    async fn call<'t, 'r, R, Ident, T>(&self, ident: Ident, args: T) -> Result<R, Error>
    where
        Ident: Into<MemberIdent> + Send,
        T: IntoValue<'t> + Send,
        R: FromValue<'r>,
    {
        Ok(self
            .meta_call(ident.into(), args.into_value())
            .await?
            .cast_into()
            .map_err(Error::MethodReturnValue)?)
    }

    async fn property<'r, Ident, R>(&self, ident: Ident) -> Result<R, Error>
    where
        Ident: Into<MemberIdent> + Send,
        R: Reflect + FromValue<'r>,
    {
        self.call(ACTION_ID_PROPERTY, Dynamic(ident.into())).await
    }

    async fn set_property<'t, Ident, T>(&self, ident: Ident, value: T) -> Result<(), Error>
    where
        Ident: Into<MemberIdent> + Send,
        T: IntoValue<'t> + Send,
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
impl<O> ObjectExt for O where O: Object + Sync + ?Sized {}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Uid(qi_value::object::ObjectUid);

impl Uid {
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(qi_value::object::ObjectUid::from_bytes(bytes))
    }

    pub fn from_ptr<T: ?Sized>(ptr: *const T) -> Self {
        let machine_id = os::MachineId::default();
        let process_uuid = os::process_uuid();
        let ptr_addr = ptr.cast::<()>() as usize;
        let digest = <Sha1 as Digest>::new()
            .chain_update(machine_id.as_bytes())
            .chain_update(process_uuid.as_bytes())
            .chain_update(ptr_addr.to_ne_bytes())
            .finalize();
        Self::from_bytes(digest.into())
    }
}

impl std::fmt::Display for Uid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl qi_value::Reflect for Uid {
    fn ty() -> Option<qi_value::Type> {
        Some(qi_value::Type::String)
    }
}

impl qi_value::RuntimeReflect for Uid {
    fn ty(&self) -> qi_value::Type {
        qi_value::Type::String
    }
}

impl qi_value::ToValue for Uid {
    fn to_value(&self) -> qi_value::Value<'_> {
        qi_value::Value::ByteString(Cow::Borrowed(self.0.bytes()))
    }
}

impl<'a> qi_value::IntoValue<'a> for Uid {
    fn into_value(self) -> qi_value::Value<'a> {
        qi_value::Value::ByteString(Cow::Owned(self.0.bytes().to_vec()))
    }
}

impl<'a> qi_value::FromValue<'a> for Uid {
    fn from_value(
        value: qi_value::Value<'a>,
    ) -> std::result::Result<Self, qi_value::FromValueError> {
        let bytes =
            value
                .as_string_bytes()
                .ok_or_else(|| qi_value::FromValueError::TypeMismatch {
                    expected: "an Object UID".to_owned(),
                    actual: value.to_string(),
                })?;
        let bytes = <[u8; 20]>::try_from(bytes)
            .map_err(|err| qi_value::FromValueError::Other(err.into()))?;
        Ok(Self(qi_value::object::ObjectUid::from_bytes(bytes)))
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    service_id: ServiceId,
    id: Id,
    uid: Uid,
    meta: MetaObject,
    session: Session,
}

impl Client {
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
        let meta = fetch_meta_object(&session, service_id, id).await?;
        Ok(Self {
            service_id,
            id,
            uid,
            meta,
            session,
        })
    }
}

#[async_trait]
impl Object for Client {
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
        let args_signature = args.signature();
        Ok(self
            .session
            .call(
                message::Address(self.service_id, self.id, method.uid),
                args,
                method.return_signature.to_type(),
            )
            .await?)
    }

    async fn meta_post(&self, ident: MemberIdent, args: Value<'_>) -> Result<(), Error> {
        let target = PostTarget::get(&self.meta, &ident)?;
        let args_signature = args.signature();
        let target_signature = target.parameters_signature();
        Ok(self
            .session
            .oneway(
                message::Address(self.service_id, self.id, target.action_id()),
                message::Oneway::Post(args),
            )
            .await?)
    }

    async fn meta_event(&self, ident: MemberIdent, value: Value<'_>) -> Result<(), Error> {
        let signal = self
            .meta
            .signal(&ident)
            .ok_or_else(|| Error::SignalNotFound(ident))?;
        let args_signature = value.signature();
        Ok(self
            .session
            .oneway(
                message::Address(self.service_id, self.id, signal.uid),
                message::Oneway::Event(value),
            )
            .await?)
    }

    fn uid(&self) -> Uid {
        self.uid
    }
}

pub(super) async fn fetch_meta_object(
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
        .cast_into()?)
}

#[derive(Debug)]
pub(super) enum PostTarget<'a> {
    Method(&'a MetaMethod),
    Signal(&'a MetaSignal),
}

impl<'a> PostTarget<'a> {
    pub(super) fn get(meta: &'a MetaObject, ident: &MemberIdent) -> Result<Self, Error> {
        meta.method(ident)
            .map(Self::Method)
            .or_else(|| meta.signal(ident).map(Self::Signal))
            .ok_or_else(|| Error::MethodOrSignalNotFound(ident.clone()))
    }

    fn action_id(&self) -> ActionId {
        match self {
            PostTarget::Method(method) => method.uid,
            PostTarget::Signal(signal) => signal.uid,
        }
    }

    pub(super) fn parameters_signature(&self) -> &value::Signature {
        match self {
            PostTarget::Method(method) => &method.parameters_signature,
            PostTarget::Signal(signal) => &signal.signature,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("failed to convert arguments")]
    Arguments(#[source] value::FromValueError),

    #[error("failed to convert of method return value")]
    MethodReturnValue(#[source] value::FromValueError),

    #[error("no object method with identifier {0}")]
    MethodNotFound(MemberIdent),

    #[error("no object signal with identifier {0}")]
    SignalNotFound(MemberIdent),

    #[error("no object method or signal with identifier {0}")]
    MethodOrSignalNotFound(MemberIdent),

    #[error("method returned an error")]
    Method(#[source] Box<dyn std::error::Error + Send + Sync>),
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
                    let arg = args.cast_into().map_err(Error::Arguments)?;
                    calc.add(arg).into_value()
                }
                Self::Sub => {
                    let arg = args.cast_into().map_err(Error::Arguments)?;
                    calc.sub(arg).into_value()
                }
                Self::Mul => {
                    let arg = args.cast_into().map_err(Error::Arguments)?;
                    calc.mul(arg).into_value()
                }
                Self::Div => {
                    let arg = args.cast_into().map_err(Error::Arguments)?;
                    calc.div(arg)
                        .map_err(|err| Error::Method(err.into()))?
                        .into_value()
                }
                Self::Clamp => {
                    let (arg1, arg2) = args.cast_into().map_err(Error::Arguments)?;
                    calc.clamp(arg1, arg2).into_value()
                }
                Self::Ans => {
                    let () = args.cast_into().map_err(Error::Arguments)?;
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

        async fn meta_post(&self, ident: MemberIdent, args: Value<'_>) -> Result<(), Error> {
            self.meta_call(ident, args).await?;
            Ok(())
        }

        async fn meta_event(&self, ident: MemberIdent, _value: Value<'_>) -> Result<(), Error> {
            Err(Error::SignalNotFound(ident))
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
        assert_matches!(res,
            Err(Error::Method(err)) => {
                assert_eq!(err.to_string(), "division by zero");
            }
        );
        let res: Result<i32, _> = calc.call("log", 1).await;
        assert_matches!(
            res,
            Err(Error::SignalNotFound(ident)) => assert_eq!(ident, "log")
        );
        let res: i32 = calc.call("ans", ()).await.unwrap();
        assert_eq!(res, 127);
    }
}
