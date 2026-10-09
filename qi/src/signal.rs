use std::sync::{Arc, Mutex};

use crate::object::ObjectClient;
use qi_value::{ActionId, Value};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value(crate = "crate::value", transparent))]
pub struct SignalLink(u64);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Event {
    uid: ActionId,
    value: Value<'static>,
}

type Listener<T> = Arc<dyn Fn(T) + Send + Sync>;

enum SignalKind<T> {
    Local(Arc<Mutex<Vec<Listener<T>>>>),
    Remote { client: ObjectClient, name: String },
}

pub struct Signal<T> {
    kind: SignalKind<T>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            kind: match &self.kind {
                SignalKind::Local(listeners) => SignalKind::Local(Arc::clone(listeners)),
                SignalKind::Remote { client, name } => SignalKind::Remote {
                    client: client.clone(),
                    name: name.clone(),
                },
            },
        }
    }
}

impl<T: Clone> Default for Signal<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Signal<T> {
    pub fn new() -> Self {
        Self {
            kind: SignalKind::Local(Arc::new(Mutex::new(Vec::new()))),
        }
    }

    pub fn remote(client: ObjectClient, name: impl Into<String>) -> Self {
        Self {
            kind: SignalKind::Remote {
                client,
                name: name.into(),
            },
        }
    }

    pub fn emit(&self, value: T) {
        let SignalKind::Local(listeners) = &self.kind else {
            return;
        };
        let listeners = listeners.lock().unwrap().clone();
        for listener in listeners {
            listener(value.clone()); // FIXME: Remove this .clone() by allowing to pass a reference to each listener
        }
    }

    pub fn subscribe(&self, listener: impl Fn(T) + Send + Sync + 'static) {
        let SignalKind::Local(listeners) = &self.kind else {
            return;
        };
        listeners.lock().unwrap().push(Arc::new(listener));
    }
}
