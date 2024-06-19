use crate::{
    messaging::{self, message},
    object, service,
    value::BinaryValue,
    BoxObject, Error, Result,
};
use qi_messaging::BodyBuf;
use qi_value::object::MemberAddress;
use std::collections::{hash_map, HashMap};
use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;

#[derive(Default, Debug)]
pub(super) struct ServiceMap(HashMap<service::Id, ServiceData>);

impl ServiceMap {
    pub(super) fn insert(&mut self, name: String, info: service::Info, main_object: BoxObject) {
        self.0
            .insert(info.id(), ServiceData::new(name, info, main_object));
    }

    pub(super) fn info_mut(&mut self) -> impl Iterator<Item = &mut service::Info> {
        self.0.values_mut().map(|data| &mut data.info)
    }

    pub(super) async fn call(
        &mut self,
        address: message::Address,
        mut args: BinaryValue,
    ) -> Result<BinaryValue> {
        let (object, address) = self.get_object(address)?;
        // Get the targeted method, so that we can get the expected parameters type, and know what
        // type of value we're supposed to deserialize.
        let method = object
            .meta()
            .method(&address)
            .ok_or_else(|| Error::MethodNotFound(address.clone()))?;
        let args = args.deserialize_value_of_type(method.parameters_signature.to_type())?;
        let reply = object.meta_call(address, args).await?;
        BinaryValue::serialize(&reply)
    }

    pub(super) async fn post(
        &mut self,
        address: message::Address,
        mut value: BinaryValue,
    ) -> Result<()> {
        let (object, address) = self.get_object(address)?;
        // Same as for "call", we need to know the type of parameters to know what to deserialize.
        let target = object::PostTarget::get(object.meta(), &address)?;
        let target_signature = target.parameters_signature();
        let value = value.deserialize_value_of_type(target_signature.to_type())?;
        object.meta_post(address, value).await
    }

    pub(super) async fn event(
        &mut self,
        address: message::Address,
        mut value: BinaryValue,
    ) -> Result<()> {
        let (object, address) = self.get_object(address)?;
        let signal = object
            .meta()
            .signal(&address)
            .ok_or_else(|| Error::SignalNotFound(address.clone()))?;
        let value = value.deserialize_value_of_type(signal.signature.to_type())?;
        object.meta_event(address, value).await
    }

    fn get_object(&self, address: message::Address) -> Result<(&BoxObject, MemberAddress)> {
        let message::Address(service_id, object_id, action_id) = address;
        let data = self
            .0
            .get(&service_id)
            .ok_or_else(|| Error::NoMessageHandler(message::Type::Call, address))?;
        let object = data
            .objects
            .get(&object_id)
            .ok_or_else(|| Error::NoMessageHandler(message::Type::Call, address))?;
        Ok((object, action_id.into()))
    }
}

#[derive(Default)]
pub(super) struct ServiceData {
    name: String,
    info: service::Info,
    objects: HashMap<object::Id, BoxObject>,
}

impl ServiceData {
    pub(super) fn new(name: String, info: service::Info, main_object: BoxObject) -> Self {
        Self {
            name,
            info,
            objects: [(service::MAIN_OBJECT_ID, main_object)]
                .into_iter()
                .collect(),
        }
    }
}

impl std::fmt::Debug for ServiceData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceData")
            .field("name", &self.name)
            .field("info", &self.info)
            .finish()
    }
}

#[derive(Default)]
pub(super) struct PendingServiceMap(HashMap<String, BoxObject>);

impl PendingServiceMap {
    pub(super) fn add(&mut self, name: String, object: BoxObject) {
        self.0.insert(name, object);
    }
}

impl std::fmt::Debug for PendingServiceMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.keys()).finish()
    }
}

impl IntoIterator for PendingServiceMap {
    type Item = (String, BoxObject);
    type IntoIter = hash_map::IntoIter<String, BoxObject>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct MessagingHandler(Arc<Mutex<ServiceMap>>);

impl MessagingHandler {
    pub(super) fn new(service_map: Arc<Mutex<ServiceMap>>) -> Self {
        Self(service_map)
    }
}

impl messaging::Handler<BinaryValue> for MessagingHandler {
    type Reply = BinaryValue;
    type Error = Error;

    fn call(
        &self,
        address: message::Address,
        value: BinaryValue,
    ) -> impl Future<Output = Result<Self::Reply>> + Send {
        let service_map = Arc::clone(&self.0);
        async move { service_map.lock_owned().await.call(address, value).await }
    }

    fn oneway(
        &self,
        address: message::Address,
        request: message::Oneway<BinaryValue>,
    ) -> impl Future<Output = Result<()>> + Send {
        let service_map = Arc::clone(&self.0);
        async move {
            let mut service_map = service_map.lock_owned().await;
            match request {
                message::Oneway::Post(value) => service_map.post(address, value).await,
                message::Oneway::Event(value) => service_map.event(address, value).await,
                message::Oneway::Capabilities(_) => {
                    // Capabilities messages are not handled by nodes services and messaging handler.
                    Err(Error::NoMessageHandler(
                        message::Type::Capabilities,
                        address,
                    ))
                }
            }
        }
    }
}
