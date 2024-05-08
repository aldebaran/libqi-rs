use crate::{service, Object};
use qi_value::{ObjectId, ServiceId};
use std::{collections::HashMap, sync::Arc};

pub(super) type Registered = HashMap<ServiceId, (String, Objects)>;
type ArcObject = Arc<dyn Object + Send + Sync>;

#[derive(Default)]
pub(super) struct Objects(HashMap<ObjectId, ArcObject>);

impl Objects {
    pub(super) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(super) fn add_main_object(&mut self, object: ArcObject) {
        self.0.insert(service::MAIN_OBJECT_ID, object);
    }

    pub(super) fn get(&self, id: &ObjectId) -> Option<&ArcObject> {
        self.0.get(id)
    }
}

impl std::fmt::Debug for Objects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.0.keys().map(|id| (id, "Object")))
            .finish()
    }
}
