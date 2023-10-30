use qi_value::object::ServiceId;

#[derive(Debug)]
pub(crate) struct ServiceRegistry;

impl ServiceRegistry {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn insert<O>(&mut self, id: ServiceId, object: O) -> Result<(), InsertError> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InsertError {}
