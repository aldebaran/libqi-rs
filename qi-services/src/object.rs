// pub mod any;

use crate::error::{Error, NoSuchMethodError};
use async_trait::async_trait;
use qi_value::{object::MetaObject, ActionId, FromValue, IntoValue, Value};

#[async_trait]
pub trait Object {
    async fn meta_object(&mut self) -> Result<MetaObject, Error>;

    async fn call_with_id(
        &mut self,
        id: ActionId,
        args: Value<'_>,
    ) -> Result<Value<'static>, Error>;

    async fn property_with_id(&mut self, id: ActionId) -> Option<Value<'static>>;

    async fn set_property_with_id(&mut self, id: ActionId, value: Value<'_>) -> bool;

    async fn property(&mut self, name: &str) -> Option<Value<'static>> {
        match self
            .meta_object()
            .await
            .ok()
            .and_then(|meta| meta.property(name).map(|(&id, _)| id))
        {
            Some(id) => self.property_with_id(id).await,
            None => None,
        }
    }

    async fn set_property(&mut self, name: &str, value: Value<'static>) -> bool {
        match self
            .meta_object()
            .await
            .ok()
            .and_then(|meta| meta.property(name).map(|(&id, _)| id))
        {
            Some(id) => self.set_property_with_id(id, value).await,
            None => false,
        }
    }

    async fn properties(&mut self) -> Result<Vec<String>, Error> {
        Ok(self
            .meta_object()
            .await?
            .properties
            .into_iter()
            .map(|(_uid, prop)| prop.name)
            .collect())
    }

    async fn call<'t, 'r, R, T>(&mut self, name: &str, args: T) -> Result<R, Error>
    where
        T: IntoValue<'t> + Send,
        R: FromValue<'r>,
    {
        let args = args.into_value();
        let (&id, _) = self
            .meta_object()
            .await?
            .method(name)
            .ok_or_else(|| NoSuchMethodError::Name(name.to_owned()))?;
        let call = self.call_with_id(id, args);
        let return_value = call.await?.cast()?;
        Ok(return_value)
    }
}
