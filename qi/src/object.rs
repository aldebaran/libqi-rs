use crate::{error::Error, os, session};
use async_trait::async_trait;
use qi_messaging::message;
pub use qi_value::{
    object::{MemberAddress, MetaObject},
    ObjectId as Id,
};
use qi_value::{ActionId, Dynamic, FromValue, IntoValue, Reflect, ServiceId, Value};
use sealed::sealed;
use sha1::{Digest, Sha1};
use std::borrow::Cow;

#[async_trait]
pub trait Object {
    fn meta_object(&self) -> MetaObject;

    async fn meta_call(
        &self,
        address: MemberAddress,
        args: Value<'_>,
    ) -> Result<Value<'static>, Error>;

    async fn meta_property(&self, address: MemberAddress) -> Result<Value<'static>, Error>;

    async fn meta_set_property(
        &self,
        address: MemberAddress,
        value: Value<'_>,
    ) -> Result<(), Error>;

    fn uid(&self) -> Uid {
        Uid::from_ptr(self)
    }
}

pub type BoxObject<'a> = Box<dyn Object + Send + Sync + 'a>;

#[sealed]
#[async_trait]
pub trait ObjectExt: Object + Sync {
    async fn property<'r, A, R>(&self, address: A) -> Result<R, Error>
    where
        A: Into<MemberAddress> + Send,
        R: Reflect + FromValue<'r>,
    {
        Ok(self.meta_property(address.into()).await?.cast_into()?)
    }

    async fn set_property<'t, A, T>(&self, address: A, value: T) -> Result<(), Error>
    where
        A: Into<MemberAddress> + Send,
        T: IntoValue<'t> + Send,
    {
        Ok(self
            .meta_set_property(address.into(), value.into_value())
            .await?)
    }

    async fn properties(&self) -> Result<Vec<String>, Error> {
        Ok(self
            .meta_object()
            .properties
            .into_iter()
            .map(|(_uid, prop)| prop.name)
            .collect())
    }

    async fn call<'t, 'r, R, A, T>(&self, address: A, args: T) -> Result<R, Error>
    where
        A: Into<MemberAddress> + Send,
        T: IntoValue<'t> + Send,
        R: FromValue<'r>,
    {
        Ok(self
            .meta_call(address.into(), args.into_value())
            .await?
            .cast_into()?)
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
    fn from_value(value: qi_value::Value<'a>) -> Result<Self, qi_value::FromValueError> {
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
    session: session::Client,
}

impl Client {
    pub(crate) fn new(
        service_id: ServiceId,
        id: Id,
        uid: Uid,
        meta: MetaObject,
        session: session::Client,
    ) -> Self {
        Self {
            service_id,
            id,
            uid,
            session,
            meta,
        }
    }

    pub fn id(&self) -> session::Uid {
        self.session.uid()
    }
}

pub(crate) async fn fetch_meta(
    session: &session::Client,
    service_id: ServiceId,
    id: Id,
) -> Result<MetaObject, Error> {
    Ok(session
        .call(
            message::Address::new(service_id, id, ACTION_ID_METAOBJECT),
            0.into_value(), // unused
            MetaObject::signature().into_type(),
        )
        .await?
        .cast_into()?)
}

#[async_trait]
impl Object for Client {
    fn meta_object(&self) -> MetaObject {
        self.meta.clone()
    }

    async fn meta_call(
        &self,
        address: MemberAddress,
        args: Value<'_>,
    ) -> Result<Value<'static>, Error> {
        let method = self
            .meta
            .method(&address)
            .ok_or_else(|| Error::MethodNotFound(address))?;
        Ok(self
            .session
            .call(
                message::Address::new(self.service_id, self.id, method.uid),
                args,
                method.return_signature.to_type().cloned(),
            )
            .await?)
    }

    async fn meta_property(&self, address: MemberAddress) -> Result<Value<'static>, Error> {
        self.call(ACTION_ID_PROPERTY, Dynamic(address)).await
    }

    async fn meta_set_property(
        &self,
        address: MemberAddress,
        value: Value<'_>,
    ) -> Result<(), Error> {
        self.call(ACTION_ID_SET_PROPERTY, (Dynamic(address), Dynamic(value)))
            .await
    }

    fn uid(&self) -> Uid {
        self.uid
    }
}

// const ACTION_ID_REGISTER_EVENT: ActionId = ActionId(0);
// const ACTION_ID_UNREGISTER_EVENT: ActionId = ActionId(1);
const ACTION_ID_METAOBJECT: ActionId = ActionId(2);
// const ACTION_ID_TERMINATE: ActionId = ActionId(3);
const ACTION_ID_PROPERTY: ActionId = ActionId(5); // not a typo, there is no action 4
const ACTION_ID_SET_PROPERTY: ActionId = ActionId(6);
// const ACTION_ID_PROPERTIES: ActionId = ActionId(7);
// const ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE: ActionId = ActionId(8);
pub const ACTION_START_ID: ActionId = ActionId(100);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
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

        fn div(&mut self, b: i32) -> Result<i32, DivisionByZeroError> {
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
        fn from_address(address: &MemberAddress) -> Option<Self> {
            let Meta { object, methods } = Meta::get();
            object.method(address).and_then(|method| {
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
                    let arg = args.cast_into()?;
                    calc.add(arg).into_value()
                }
                Self::Sub => {
                    let arg = args.cast_into()?;
                    calc.sub(arg).into_value()
                }
                Self::Mul => {
                    let arg = args.cast_into()?;
                    calc.mul(arg).into_value()
                }
                Self::Div => {
                    let arg = args.cast_into()?;
                    calc.div(arg)
                        .map_err(|err| Error::Other(err.into()))?
                        .into_value()
                }
                Self::Clamp => {
                    let (arg1, arg2) = args.cast_into()?;
                    calc.clamp(arg1, arg2).into_value()
                }
                Self::Ans => {
                    args.cast_into()?;
                    calc.ans().into_value()
                }
            })
        }
    }

    #[async_trait]
    impl Object for Mutex<Calculator> {
        fn meta_object(&self) -> MetaObject {
            Meta::get().object.clone()
        }

        async fn meta_call(
            &self,
            address: MemberAddress,
            args: Value<'_>,
        ) -> Result<Value<'static>, Error> {
            Method::from_address(&address)
                .ok_or_else(|| Error::MethodNotFound(address))?
                .call(&mut *self.lock().await, args)
        }

        async fn meta_property(&self, address: MemberAddress) -> Result<Value<'static>, Error> {
            Err(Error::PropertyNotFound(address))
        }

        async fn meta_set_property(
            &self,
            address: MemberAddress,
            _value: Value<'_>,
        ) -> Result<(), Error> {
            Err(Error::PropertyNotFound(address))
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
            Err(Error::Other(err)) => {
                assert_eq!(err.to_string(), "division by zero");
            }
        );
        let res: Result<i32, _> = calc.call("log", 1).await;
        assert_matches!(
            res,
            Err(Error::MethodNotFound(name)) => assert_eq!(name, "log")
        );
        let res: i32 = calc.call("ans", ()).await.unwrap();
        assert_eq!(res, 127);
    }
}
