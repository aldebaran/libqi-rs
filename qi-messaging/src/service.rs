use crate::{Call, Error, Event, Post};
use bytes::Bytes;
use futures::future::BoxFuture;

pub trait Service {
    fn call(&self, call: Call) -> BoxFuture<'static, Result<Bytes, Error>>;
    fn post(&self, post: Post) -> BoxFuture<'static, Result<(), Error>>;
    fn event(&self, event: Event) -> BoxFuture<'static, Result<(), Error>>;
}

impl<S> Service for S
where
    S: std::ops::Deref,
    S::Target: Service,
{
    fn call(&self, call: Call) -> BoxFuture<'static, Result<Bytes, Error>> {
        (**self).call(call)
    }

    fn post(&self, post: Post) -> BoxFuture<'static, Result<(), Error>> {
        (**self).post(post)
    }

    fn event(&self, event: Event) -> BoxFuture<'static, Result<(), Error>> {
        (**self).event(event)
    }
}
