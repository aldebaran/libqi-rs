use std::sync::{atomic::AtomicU32, Arc};

use crate::message::Id;

const FIRST_ID: u32 = 1;

#[derive(Clone, Debug)]
pub(crate) struct SharedIdFactory(Arc<AtomicU32>);

impl SharedIdFactory {
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicU32::new(FIRST_ID)))
    }

    pub(crate) fn create(&self) -> Id {
        use std::sync::atomic::Ordering;
        Id(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for SharedIdFactory {
    fn default() -> Self {
        Self::new()
    }
}
