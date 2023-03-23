use crate::message;
pub use crate::req_rep::{Call, CallBuilder, Response};
use std::cell::Cell;

pub trait Session {
    fn create_call(&self) -> CallBuilder;

    fn send_call_request<T>(&self, call: Call<T>) -> CallRequest
    where
        T: Send;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct MessageIdGenerator(Cell<message::Id>);

impl MessageIdGenerator {
    fn generate(&self) -> message::Id {
        // TODO: use `Cell::update` when available.
        let mut id = self.0.get();
        id.increment();
        self.0.set(id);
        id
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Inner<S> {
    stream: S,
    #[new(default)]
    msg_id_gen: MessageIdGenerator,
}

impl<S> Session for Inner<S> {
    fn create_call(&self) -> CallBuilder {
        CallBuilder::new(self.msg_id_gen.generate())
    }

    fn send_call_request<T>(&self, call: Call<T>) -> CallRequest
    where
        T: Send,
    {
        todo!()
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CallRequest;

#[cfg(test)]
mod tests {}
