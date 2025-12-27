use bytes::Bytes;

use crate::{message, value::KeyDynValueMap};
use std::{convert::Infallible, future::Future};

pub trait CallHandler {
    type Error: CallError + Send + 'static;
    type Future: Future<Output = Result<Bytes, Self::Error>> + Send + 'static;
    fn handle_call(&self, address: message::Address, args: Bytes) -> Self::Future;
}

pub trait EventHandler {
    fn handle_event(&self, address: message::Address, args: Bytes);
}

pub trait PostHandler {
    fn handle_post(&self, address: message::Address, args: Bytes);
}

pub trait CapabilitiesHandler {
    fn handle_capabilities(&self, address: message::Address, data: KeyDynValueMap);
}

pub trait Handler: CallHandler + EventHandler + PostHandler + CapabilitiesHandler {}

impl<T> Handler for T where T: CallHandler + EventHandler + PostHandler + CapabilitiesHandler {}

/// An call handler error that is able to signify handling conditions to the messaging loop.
pub trait CallError: std::fmt::Display {
    /// The error is a consequence of a request cancellation. The messaging loop must notify the
    /// client that the request has been canceled.
    fn is_canceled(&self) -> bool;

    /// The error is fatal to the messaging loop. The loop must send the error back to the client
    /// and then terminate.
    fn is_fatal(&self) -> bool;
}

impl CallError for Infallible {
    fn is_canceled(&self) -> bool {
        false
    }

    fn is_fatal(&self) -> bool {
        false
    }
}
