use crate::{messaging::{self, message}, service, Error, Object};
use bytes::Bytes;
use qi_value::{ObjectId, ServiceId};
use std::{collections::HashMap, sync::Arc};

pub(super) struct RegisteredServicesMap(HashMap<ServiceId, (String, Objects)>);

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



impl tower::Service<(message::Address, Bytes)> for RegisteredServicesMap {
    type Response = Bytes;
    type Error = Error;
    type Future = CallFuture;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, (address, value): (message::Address, Bytes)) -> Self::Future {
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
}

type ArcObject = Arc<dyn Object + Send + Sync>;

pub(super) struct CallFuture;

impl std::future::Future for CallFuture {
    type Output = Result<Bytes, Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        todo!()
    }
}
