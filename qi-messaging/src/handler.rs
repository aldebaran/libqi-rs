use crate::message;
use std::future::Future;

pub trait Handler<T> {
    type Error;
    type Reply;

    fn call(
        &self,
        address: message::Address,
        value: T,
    ) -> impl Future<Output = Result<Self::Reply, Self::Error>> + Send;

    fn oneway(
        &self,
        address: message::Address,
        request: message::Oneway<T>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
