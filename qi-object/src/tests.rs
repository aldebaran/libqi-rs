use crate::{
    object::{
        error::{
            AnyCallError, CallError, NoSuchMethodError, NoSuchPropertyError, NoSuchSignalError,
        },
        IntoObject, Object,
    },
    signal::SignalLink,
};
use assert_matches::assert_matches;
use async_trait::async_trait;
use qi_type::Signature;
use qi_value::{
    object::{ActionId, MetaObject, ObjectId},
    Dynamic,
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

    fn ans(&self) -> i32 {
        self.a
    }
}

#[derive(Debug, thiserror::Error)]
#[error("division by zero")]
struct DivisionByZeroError;

impl IntoObject for Calculator {
    type Object = CalculatorObject;

    fn into_object(self) -> Self::Object {
        CalculatorObject::new(self)
    }
}

#[derive(Debug)]
struct CalculatorObject {
    implem: Calculator,
    meta: MetaObject,
}

impl CalculatorObject {
    fn new(implem: Calculator) -> Self {
        let meta = MetaObject::builder()
            .add_method(
                ActionId(0),
                "add".to_owned(),
                "(i)".parse().unwrap(),
                "i".parse().unwrap(),
            )
            .add_method(
                ActionId(1),
                "sub".to_owned(),
                "(i)".parse().unwrap(),
                "i".parse().unwrap(),
            )
            .add_method(
                ActionId(2),
                "mul".to_owned(),
                "(i)".parse().unwrap(),
                "i".parse().unwrap(),
            )
            .add_method(
                ActionId(3),
                "div".to_owned(),
                "(i)".parse().unwrap(),
                "i".parse().unwrap(),
            )
            .add_method(
                ActionId(4),
                "ans".to_owned(),
                "()".parse().unwrap(),
                "i".parse().unwrap(),
            )
            .build();
        Self { implem, meta }
    }
}

#[async_trait]
impl Object for CalculatorObject {
    async fn register_event(
        &mut self,
        event: ActionId,
        link: SignalLink,
    ) -> Result<SignalLink, CallError> {
        Err(CallError::Other(NoSuchSignalError::Id(event).into()))
    }

    async fn register_event_with_signature(
        &mut self,
        event: ActionId,
        link: SignalLink,
        signature: Signature,
    ) -> Result<SignalLink, CallError> {
        Err(CallError::Other(NoSuchSignalError::Id(event).into()))
    }

    async fn unregister_event(
        &mut self,
        object: ObjectId,
        event: ActionId,
        link: SignalLink,
    ) -> Result<(), CallError> {
        Err(CallError::Other(NoSuchSignalError::Id(event).into()))
    }

    async fn meta_object(&mut self) -> Result<MetaObject, CallError> {
        Ok(self.meta)
    }

    async fn property<T>(&mut self, name: Dynamic<&str>) -> Result<Dynamic<T>, CallError> {
        Err(CallError::Other(
            NoSuchPropertyError::Name(name.0.to_owned()).into(),
        ))
    }

    async fn set_property<T>(
        &mut self,
        name: Dynamic<&str>,
        value: Dynamic<T>,
    ) -> Result<(), CallError>
    where
        T: Send,
    {
        Err(CallError::Other(
            NoSuchPropertyError::Name(name.0.to_owned()).into(),
        ))
    }

    async fn properties(&self) -> Result<Vec<String>, CallError> {
        Ok(vec![])
    }
}

#[derive(Debug, thiserror::Error)]
enum CalculatorObjectError {
    #[error("format error")]
    Format(#[from] qi_format::Error),

    #[error(transparent)]
    DivisionByZero(#[from] DivisionByZeroError),

    #[error(transparent)]
    NoSuchMethod(#[from] NoSuchMethodError),
}

#[tokio::test]
async fn test_calculator_into_object_meta_call() {
    let calc = Calculator::new(42);
    let object = calc.into_object();
    assert_eq!(object.call("add", 100).await.unwrap(), 142);
    assert_eq!(object.call("add", 50).await.unwrap(), 192);
    assert_eq!(object.call("sub", 12).await.unwrap(), 180);
    assert_eq!(object.call("div", 90).await.unwrap(), 2);
    assert_eq!(object.call("mul", 64).await.unwrap(), 128);
    assert_matches!(
        object.call("div", 0).await,
        Err(AnyCallError::Other(err)) => {
            assert_eq!(err.to_string(), "division by zero");
        }
    );
    assert_eq!(object.call("ans", 64).await.unwrap(), 128);
}
