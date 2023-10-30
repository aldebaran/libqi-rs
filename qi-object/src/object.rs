pub mod error;

use crate::signal::SignalLink;
use async_trait::async_trait;
use error::{AnyCallError, CallError};
use futures::{Sink, SinkExt};
use qi_type::Signature;
use qi_value::{
    object::{ActionId, MetaObject, ObjectId},
    Dynamic, Value,
};
use sealed::sealed;
use tower::{Service, ServiceExt};

use self::error::NoSuchMethodError;

#[async_trait]
pub trait Object {
    async fn register_event(
        &mut self,
        event: ActionId,
        link: SignalLink,
    ) -> Result<SignalLink, CallError>;

    async fn register_event_with_signature(
        &mut self,
        event: ActionId,
        link: SignalLink,
        signature: Signature,
    ) -> Result<SignalLink, CallError>;

    async fn unregister_event(
        &mut self,
        object: ObjectId,
        event: ActionId,
        link: SignalLink,
    ) -> Result<(), CallError>;

    async fn meta_object(&mut self) -> Result<MetaObject, CallError>;

    async fn property<T>(&mut self, name: Dynamic<&str>) -> Result<Dynamic<T>, CallError>;

    async fn set_property<T>(
        &mut self,
        name: Dynamic<&str>,
        value: Dynamic<T>,
    ) -> Result<(), CallError>
    where
        T: Send;

    async fn properties(&self) -> Result<Vec<String>, CallError>;
}

#[sealed]
#[async_trait]
pub trait MetaCall {
    type Output;
    type Error;

    async fn call<T>(&mut self, name: &str, arg: T) -> Result<Self::Output, Self::Error>
    where
        T: 'async_trait + serde::Serialize;
}

#[sealed]
#[async_trait]
impl<O> MetaCall for O
where
    O: Object + tower::Service<Call> + Send,
    O::Future: Send,
    O::Error: Send,
{
    type Output = O::Response;
    type Error = AnyCallError<O::Error>;

    async fn call<T>(&mut self, name: &str, arg: T) -> Result<Self::Output, Self::Error>
    where
        T: 'async_trait + AsValue,
    {
        let meta = self.meta_object().await.map_err(AnyCallError::MetaObject)?;
        let (&action, _) = meta
            .method(name)
            .ok_or_else(|| NoSuchMethodError::Name(name.to_owned()))?;
        self.map_err(AnyCallError::Service)
            .ready()
            .await?
            .call(Call {
                action,
                arg: arg.into(),
            })
            .await
    }
}

#[derive(Debug)]
pub struct Call {
    action: ActionId,
    arg: Value,
}

#[sealed]
#[async_trait]
pub trait MetaPost {
    async fn post<T>(&mut self, name: &str, arg: T)
    where
        T: 'async_trait + serde::Serialize;
}

#[sealed]
#[async_trait]
impl<O> MetaPost for O
where
    O: Object + Sink<Post> + Send + Unpin,
{
    async fn post<T>(&mut self, name: &str, arg: T)
    where
        T: 'async_trait + serde::Serialize,
    {
        let arg = Value::serialize(&arg);
        if let Ok(meta) = self.meta_object().await {
            if let Some(&action) = meta
                .signal(name)
                .map(|(id, _)| id)
                .or_else(|| meta.method(name).map(|(id, _)| id))
            {
                let _res = self.send(Post { action, arg }).await;
            }
        }
    }
}

#[derive(Debug)]
pub struct Post {
    action: ActionId,
    arg: Value,
}

pub(crate) trait IntoObject {
    type Object: Object;

    fn into_object(self) -> Self::Object;
}
