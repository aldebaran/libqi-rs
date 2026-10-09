use std::sync::{Arc, Mutex};

use crate::{object::ObjectClient, signal::Signal};

enum PropertyKind<T> {
    Local {
        value: Arc<Mutex<T>>,
        signal: Signal<T>,
    },
    Remote {
        client: ObjectClient,
        name: String,
    },
}

pub struct Property<T> {
    kind: PropertyKind<T>,
}

impl<T> Clone for Property<T> {
    fn clone(&self) -> Self {
        Self {
            kind: match &self.kind {
                PropertyKind::Local { value, signal } => PropertyKind::Local {
                    value: Arc::clone(value),
                    signal: signal.clone(),
                },
                PropertyKind::Remote { client, name } => PropertyKind::Remote {
                    client: client.clone(),
                    name: name.clone(),
                },
            },
        }
    }
}

impl<T: Clone> Property<T> {
    pub fn new(value: T) -> Self {
        Self {
            kind: PropertyKind::Local {
                value: Arc::new(Mutex::new(value)),
                signal: Signal::new(),
            },
        }
    }

    pub fn remote(client: ObjectClient, name: impl Into<String>) -> Self {
        Self {
            kind: PropertyKind::Remote {
                client,
                name: name.into(),
            },
        }
    }

    pub fn get(&self) -> T {
        match &self.kind {
            PropertyKind::Local { value, .. } => value.lock().unwrap().clone(),
            PropertyKind::Remote { .. } => {
                unreachable!("remote property reads go through the object client")
            }
        }
    }

    /// Stores `value` and emits it on the signal entry registered for this property.
    pub fn set(&self, value: T) {
        let PropertyKind::Local {
            value: slot,
            signal,
        } = &self.kind
        else {
            return;
        };
        *slot.lock().unwrap() = value.clone();
        signal.emit(value);
    }

    pub fn subscribe(&self, listener: impl Fn(T) + Send + Sync + 'static) {
        let PropertyKind::Local { signal, .. } = &self.kind else {
            return;
        };
        signal.subscribe(listener);
    }
}
