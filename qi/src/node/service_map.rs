use crate::{binary_value::BinaryValue, messaging::message, service, BoxObject, Error, Result};
use qi_value::{ObjectId, ServiceId};
use std::collections::HashMap;

#[derive(Default, Debug)]
pub(super) struct ServiceMap {
    services: HashMap<ServiceId, ServiceObjects>,
}

impl ServiceMap {
    pub(super) async fn call(
        &mut self,
        address: message::Address,
        value: BinaryValue,
    ) -> Result<BinaryValue> {
        todo!()
        // let address = call.address();
        // let service_id = address.service();
        // let res = match self.services.get(&service_id) {
        //     Some((_, objects)) => {
        //         let object_id = address.object();
        //         match objects.get(&object_id) {
        //             Some(object) => {
        //                 let method_address = address.action().into();
        //                 match object.meta().method(&method_address) {
        //                     Some(method) => call
        //                         .into_value()
        //                         .deserialize_value_of_type(method.parameters_signature.to_type())
        //                         .map(|args| {
        //                             let object = Arc::clone(object);
        //                             async move {
        //                                 Ok(object
        //                                     .meta_call(method_address, args)
        //                                     .await?
        //                                     .serialize_value()?)
        //                             }
        //                         })
        //                         .map_err(Into::into),
        //                     None => Err(format!(
        //                         "method {method_address} not found \
        //                         in object {object_id} in service {service_id}"
        //                     )
        //                     .into()),
        //                 }
        //             }
        //             None => {
        //                 Err(format!("object ${object_id} not found in service {service_id}").into())
        //             }
        //         }
        //     }
        //     None => Err(format!("service ${service_id} not found").into()),
        // };

        // match res {
        //     Ok(call) => call.boxed(),
        //     Err(err) => future::err(messaging::Error::Other(err)).boxed(),
        // }
    }

    pub(super) fn post(&mut self, address: message::Address, value: BinaryValue) -> Result<()> {
        todo!()
    }

    pub(super) fn event(&mut self, address: message::Address, value: BinaryValue) -> Result<()> {
        todo!()
    }
}

#[derive(Default)]
struct ServiceObjects {
    name: String,
    objects: HashMap<ObjectId, BoxObject>,
}

impl ServiceObjects {
    pub(super) fn new(name: String, main_object: BoxObject) -> Self {
        Self {
            name,
            objects: [(service::MAIN_OBJECT_ID, main_object)]
                .into_iter()
                .collect(),
        }
    }
}

impl std::fmt::Debug for ServiceObjects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceObjects")
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Default)]
pub(super) struct PendingServiceMap(HashMap<String, BoxObject>);

impl PendingServiceMap {
    pub(super) fn add(&mut self, name: String, object: BoxObject) -> Result<()> {
        use std::collections::hash_map::Entry;
        match self.0.entry(name) {
            Entry::Occupied(entry) => Err(Error::ServiceExists(entry.key().clone())),
            Entry::Vacant(entry) => {
                entry.insert(object);
                Ok(())
            }
        }
    }

    pub(super) fn remove(&mut self, name: &str) -> Option<BoxObject> {
        self.0.remove(name)
    }
}

impl std::fmt::Debug for PendingServiceMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.keys()).finish()
    }
}
