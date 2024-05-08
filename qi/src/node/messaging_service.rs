use super::services;
use crate::{messaging, Error};
use bytes::Bytes;
use futures::{
    future,
    FutureExt,
};
use messaging::message;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct MessagingService {
    services: services::Registered,
}

impl MessagingService {
    pub(super) fn new(services: services::Registered) -> Self {
        Self { services }
    }
}

impl tower::Service<(message::Address, Bytes)> for MessagingService {
    type Response = Bytes;
    type Error = Error;
    type Future = CallFuture;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, (address, value) = (message::Address, Bytes)) -> Self::Future {
        let address = call.address();
        let service_id = address.service();
        let res = match self.services.get(&service_id) {
            Some((_, objects)) => {
                let object_id = address.object();
                match objects.get(&object_id) {
                    Some(object) => {
                        let method_address = address.action().into();
                        match object.meta().method(&method_address) {
                            Some(method) => call
                                .into_value()
                                .deserialize_value_of_type(method.parameters_signature.to_type())
                                .map(|args| {
                                    let object = Arc::clone(object);
                                    async move {
                                        Ok(object
                                            .meta_call(method_address, args)
                                            .await?
                                            .serialize_value()?)
                                    }
                                })
                                .map_err(Into::into),
                            None => Err(format!(
                                "method {method_address} not found \
                                in object {object_id} in service {service_id}"
                            )
                            .into()),
                        }
                    }
                    None => {
                        Err(format!("object ${object_id} not found in service {service_id}").into())
                    }
                }
            }
            None => Err(format!("service ${service_id} not found").into()),
        };

        match res {
            Ok(call) => call.boxed(),
            Err(err) => future::err(messaging::Error::Other(err)).boxed(),
        }
    }
}

struct CallFuture {
    
}
