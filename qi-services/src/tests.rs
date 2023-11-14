use crate::{error::NoSuchMethodError, Error, Object};
use assert_matches::assert_matches;
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use qi_value::{
    object::{MetaMethod, MetaObject},
    ActionId, Type, Value,
};

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
        static META: OnceCell<Meta> = OnceCell::new();
        META.get_or_init(|| {
            let add;
            let sub;
            let mul;
            let div;
            let clamp;
            let ans;
            let mut method_id = ActionId(0);
            let mut builder = MetaObject::builder();
            builder
                .add_method({
                    add = method_id.next().unwrap();
                    let mut builder = MetaMethod::builder(add);
                    builder.set_name("add");
                    builder.parameter(0).set_type(Type::Int32);
                    builder.return_value().set_type(Type::Int32);
                    builder.build()
                })
                .add_method({
                    sub = method_id.next().unwrap();
                    let mut builder = MetaMethod::builder(sub);
                    builder.set_name("sub");
                    builder.parameter(0).set_type(Type::Int32);
                    builder.build()
                })
                .add_method({
                    mul = method_id.next().unwrap();
                    let mut builder = MetaMethod::builder(mul);
                    builder.set_name("mul");
                    builder.parameter(0).set_type(Type::Int32);
                    builder.build()
                })
                .add_method({
                    div = method_id.next().unwrap();
                    let mut builder = MetaMethod::builder(div);
                    builder.set_name("div");
                    builder.parameter(0).set_type(Type::Int32);
                    builder.build()
                })
                .add_method({
                    clamp = method_id.next().unwrap();
                    let mut builder = MetaMethod::builder(clamp);
                    builder.set_name("clamp");
                    builder.parameter(0).set_type(Type::Int32);
                    builder.parameter(1).set_type(Type::Int32);
                    builder.build()
                })
                .add_method({
                    ans = method_id.next().unwrap();
                    let mut builder = MetaMethod::builder(ans);
                    builder.set_name("ans");
                    builder.build()
                });
            let object = builder.build();
            Meta {
                methods: MethodIds {
                    add,
                    sub,
                    mul,
                    div,
                    clamp,
                    ans,
                },
                object,
            }
        })
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
    fn from_id(id: ActionId) -> Option<Self> {
        let methods = &Meta::get().methods;
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
    }
}

#[async_trait]
impl Object for Calculator {
    async fn meta_object(&mut self) -> Result<MetaObject, Error> {
        Ok(Meta::get().object.clone())
    }

    async fn call_with_id(
        &mut self,
        id: ActionId,
        args: Value<'_>,
    ) -> Result<Value<'static>, Error> {
        use qi_value::IntoValue;
        match Method::from_id(id) {
            Some(Method::Add) => {
                let arg = args.cast()?;
                let res = self.add(arg);
                let res = res.into_value();
                Ok(res)
            }
            Some(Method::Sub) => {
                let arg = args.cast()?;
                let res = self.sub(arg);
                let res = res.into_value();
                Ok(res)
            }
            Some(Method::Mul) => {
                let arg = args.cast()?;
                let res = self.mul(arg);
                let res = res.into_value();
                Ok(res)
            }
            Some(Method::Div) => {
                let arg = args.cast()?;
                let res = self.div(arg);
                let res = res.map_err(|err| Error::Other(err.into()))?;
                let res = res.into_value();
                Ok(res)
            }
            Some(Method::Clamp) => {
                let (arg1, arg2) = args.cast()?;
                let res = self.clamp(arg1, arg2);
                let res = res.into_value();
                Ok(res)
            }
            Some(Method::Ans) => {
                let () = args.cast()?;
                let res = self.ans();
                let res = res.into_value();
                Ok(res)
            }
            None => Err(Error::NoSuchMethod(NoSuchMethodError::Id(id))),
        }
    }

    async fn property_with_id(&mut self, _id: ActionId) -> Option<Value<'static>> {
        None
    }

    async fn set_property_with_id(&mut self, _id: ActionId, _value: Value<'_>) -> bool {
        false
    }
}

#[tokio::test]
async fn test_calculator_object_call_methods() {
    let mut calc = Calculator::new(42);
    assert_eq!(calc.call::<i32, _>("add", 100).await.unwrap(), 142);
    assert_eq!(calc.call::<i32, _>("add", 50).await.unwrap(), 192);
    assert_eq!(calc.call::<i32, _>("sub", 12).await.unwrap(), 180);
    assert_eq!(calc.call::<i32, _>("div", 90).await.unwrap(), 2);
    assert_eq!(calc.call::<i32, _>("mul", 64).await.unwrap(), 128);
    assert_eq!(calc.call::<i32, _>("clamp", (32, 127)).await.unwrap(), 127);
    assert_matches!(
        calc.call::<i32, _>("div", 0).await,
        Err(Error::Other(err)) => {
            assert_eq!(err.to_string(), "division by zero");
        }
    );
    assert_matches!(
        calc.call::<i32, _>("log", 1).await,
        Err(Error::NoSuchMethod(NoSuchMethodError::Name(name))) => assert_eq!(name, "log")
    );
    assert_eq!(calc.call::<i32, _>("ans", ()).await.unwrap(), 127);
}
