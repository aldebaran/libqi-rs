use crate::message;
use std::future::Future;

pub trait Handler<Body> {
    type Error;

    fn call(
        &self,
        address: message::Address,
        value: Body,
    ) -> impl Future<Output = Result<Body, Self::Error>> + Send;

    fn oneway(
        &self,
        address: message::Address,
        request: message::Oneway<Body>,
    ) -> impl Future<Output = ()> + Send;
}
