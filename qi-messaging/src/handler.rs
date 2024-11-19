use crate::message;
use std::{convert::Infallible, future::Future};

pub trait Handler<Value> {
    type Error: Error;

    fn call(
        &self,
        address: message::Address,
        value: Value,
    ) -> impl Future<Output = Result<Value, Self::Error>> + Send;

    fn fire_and_forget(
        &self,
        address: message::Address,
        request: message::FireAndForget<Value>,
    ) -> impl Future<Output = ()> + Send;
}

/// An handler error that is able to signify handling conditions to the messaging loop.
pub trait Error: std::error::Error {
    /// The error is a consequence of a request cancellation. The messaging loop must notify the
    /// client that the request has been canceled.
    fn is_canceled(&self) -> bool;

    /// The error is fatal to the messaging loop. The loop must send the error back to the client
    /// and then terminate.
    fn is_fatal(&self) -> bool;
}

impl Error for Infallible {
    fn is_canceled(&self) -> bool {
        false
    }

    fn is_fatal(&self) -> bool {
        false
    }
}
