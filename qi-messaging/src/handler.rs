use crate::message;
use std::future::Future;

pub trait Handler<T> {
    type Error;
    type Reply;
    type Future: Future<Output = Result<Self::Reply, Self::Error>>;

    fn call(&mut self, address: message::Address, value: T) -> Self::Future;

    fn oneway_request(
        &mut self,
        address: message::Address,
        request: message::OnewayRequest<T>,
    ) -> Result<(), Self::Error>;
}
