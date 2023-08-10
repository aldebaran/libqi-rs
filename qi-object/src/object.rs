pub mod client;

use crate::{
    signal,
    value::{
        self,
        object::{ActionId, MetaObject, ObjectId, ServiceId},
        Signature,
    },
    CallResult,
};
pub use client::Client;
use futures::future::BoxFuture;
use value::Value;

pub trait Object {
    type Error;

    fn register_event(
        &mut self,
        service: ServiceId,
        event: ActionId,
        link: signal::Link,
    ) -> BoxFuture<CallResult<signal::Link, Self::Error>>;

    fn register_event_with_signature(
        &mut self,
        service: ServiceId,
        event: ActionId,
        link: signal::Link,
        signature: Signature,
    ) -> BoxFuture<CallResult<signal::Link, Self::Error>>;

    fn unregister_event(
        &mut self,
        service: ServiceId,
        event: ActionId,
        link: signal::Link,
    ) -> BoxFuture<CallResult<(), Self::Error>>;

    fn meta_object(&self, id: ObjectId) -> BoxFuture<CallResult<MetaObject, Self::Error>>;

    fn property(&self, name: value::Dynamic) -> BoxFuture<CallResult<value::Dynamic, Self::Error>>;

    fn set_property(
        &mut self,
        name: value::Dynamic,
        value: value::Dynamic,
    ) -> BoxFuture<CallResult<(), Self::Error>>;

    fn properties(&self) -> BoxFuture<CallResult<Vec<String>, Self::Error>>;

    fn call<T>(
        &mut self,
        action: BoundAction,
        value: T,
    ) -> BoxFuture<CallResult<Value, Self::Error>>
    where
        T: serde::Serialize; // TODO: T: Value

    fn post<T>(&mut self, action: BoundAction, value: T) -> BoxFuture<Result<(), Self::Error>>
    where
        T: serde::Serialize; // TODO: T: Value

    fn event<T>(&mut self, action: BoundAction, value: T) -> BoxFuture<Result<(), Self::Error>>
    where
        T: serde::Serialize; // TODO: T: Value
}

#[derive(Debug)]
pub struct BoundAction(ActionId);

// static OBJECT_META_OBJECT: OnceCell<MetaObject> = OnceCell::new();
//
// fn bound_object_meta_object() -> &'static MetaObject {
//     OBJECT_META_OBJECT.get_or_init(|| {
//         let mut builder = MetaObject::builder();
//         builder.add_method(
//             ACTION_ID_REGISTER_EVENT,
//             "registerEvent",
//             ???,
//             ???,
//         );
//         builder.build()
//     })
// }
