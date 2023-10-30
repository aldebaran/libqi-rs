use super::error::CallError;
use futures::future::BoxFuture;
use qi_value::{object::ActionId, AnyValue};
use std::collections::HashMap;

#[derive(Debug)]
pub struct DynamicObject<'a> {
    methods: HashMap<ActionId, BoxMethod<'a>>,
}

struct BoxMethod<'a> {
    fun: Box<dyn FnMut(AnyValue) -> BoxFuture<'a, Result<AnyValue, CallError>> + 'a>,
}

impl<'a> BoxMethod<'a> {
    fn call_mut(&mut self, arg: AnyValue) -> BoxFuture<'a, Result<AnyValue, CallError>> {
        (self.fun)(arg)
    }
}

impl std::fmt::Debug for BoxMethod<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxMethod").finish()
    }
}
