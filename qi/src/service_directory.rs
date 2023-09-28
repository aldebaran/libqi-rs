use crate::{
    messaging::{session, CallResult},
    object,
    value::object::{ActionId, ObjectUid, ServiceId},
    Uri,
};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};

pub trait ServiceDirectory {
    fn service(&self, name: &str) -> BoxFuture<'static, CallResult<ServiceInfo, Error>>;
    fn services(&self) -> BoxFuture<'static, CallResult<Vec<ServiceInfo>, Error>>;

    // fn register_service(&mut self, info: ServiceInfo) -> Self::RegisterServiceFuture;
    // fn unregister_service(&mut self, index: ServiceId) -> Self::UnregisterServiceFuture;
    // fn service_ready(&mut self, index: ServiceId) -> Self::ServiceReadyFuture;
    // fn update_service_info(&mut self, info: ServiceInfo) -> Self::UpdateServiceInfoFuture;
    // fn machine_id(&self) -> Self::MachineIdFuture;
    // fn subscribe_service_added(&self) -> Self::SubscribeServiceFuture;
    // fn subscribe_service_removed(&self) -> Self::SubscribeServiceFuture;
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, serde::Serialize, serde::Deserialize,
)]
pub struct ServiceIdName {
    index: ServiceId,
    name: String,
}

#[derive(Debug)]
pub struct ServiceDirectoryImpl;

impl ServiceDirectory for ServiceDirectoryImpl {
    fn service(&self, name: &str) -> BoxFuture<'static, CallResult<ServiceInfo, Error>> {
        todo!()
    }

    fn services(&self) -> BoxFuture<'static, CallResult<Vec<ServiceInfo>, Error>> {
        todo!()
    }
}

const SERVICE_ID: ServiceId = ServiceId::new(1);

// struct Meta {
//     object: MetaObject,
//     actions: Actions,
// }

// static META: OnceCell<Meta> = OnceCell::new();

// struct Actions {
//     service: ActionId,
//     services: ActionId,
//     register_service: ActionId,
//     unregister_service: ActionId,
//     service_ready: ActionId,
//     update_service_info: ActionId,
//     service_added: ActionId,
//     service_removed: ActionId,
//     machine_id: ActionId,
// }

// fn actions() -> &'static Actions {
//     &META.get_or_init(|| {
//         let builder = MetaObjectBuilder::new();
//         let service = builder.add_method("service", Type::String, ???);
//         let services = todo!();
//         let register_service = todo!();
//         let unregister_service = todo!();
//         let service_ready = todo!();
//         let update_service_info = todo!();
//         let service_added = todo!();
//         let service_removed = todo!();
//         let object = builder.build();
//         let actions = Actions { service, services, register_service, unregister_service, service_ready, update_service_info, service_added, service_removed, machine_id };
//         Meta {
//             object,
//             actions
//         }
//     }).actions
// }

const ACTION_SD_SERVICE: ActionId = ActionId::new(100);
const ACTION_SD_SERVICES: ActionId = ActionId::new(101);
const ACTION_SD_REGISTER_SERVICE: ActionId = ActionId::new(102);
const ACTION_SD_UNREGISTER_SERVICE: ActionId = ActionId::new(103);
const ACTION_SD_SERVICE_READY: ActionId = ActionId::new(104);
const ACTION_SD_UPDATE_SERVICE_INFO: ActionId = ActionId::new(105);
const ACTION_SD_SERVICE_ADDED: ActionId = ActionId::new(106);
const ACTION_SD_SERVICE_REMOVED: ActionId = ActionId::new(107);
const ACTION_SD_MACHINE_ID: ActionId = ActionId::new(108);

#[derive(Debug, Clone)]
pub struct Client {
    object: object::Client,
}

impl Client {
    pub(crate) async fn connect(
        session: session::Client,
    ) -> CallResult<Self, object::client::ConnectError> {
        let object = object::Client::connect_to_service_object(session, SERVICE_ID).await?;
        Ok(Self { object })
    }
}

impl ServiceDirectory for Client {
    fn service(&self, name: &str) -> BoxFuture<'static, CallResult<ServiceInfo, Error>> {
        let call = self.object.call_action(ACTION_SD_SERVICE, name);
        call.map_err(|err| err.map_err(Error::ClientCall)).boxed()
    }

    fn services(&self) -> BoxFuture<'static, CallResult<Vec<ServiceInfo>, Error>> {
        let call = self.object.call_action(ACTION_SD_SERVICES, ());
        call.map_err(|err| err.map_err(Error::ClientCall)).boxed()
    }
}

pub type BoxServiceDirectory<'a> = Box<dyn ServiceDirectory + 'a + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ClientCall(#[from] object::client::CallError),
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default,
)]
pub struct ServiceInfo {
    pub name: String,
    pub service_id: ServiceId,
    pub machine_id: MachineId,
    pub process_id: u32,
    pub endpoints: Vec<Uri>,
    pub session_id: SessionId,
    #[serde(with = "serde_object_uid")]
    pub object_uid: Option<ObjectUid>,
}

mod serde_object_uid {
    use crate::value::object::ObjectUid;

    pub(super) fn serialize<S>(uid: &Option<ObjectUid>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::Serialize;
        match uid {
            Some(uid) => uid.serialize(serializer),
            None => serializer.serialize_bytes(&[]),
        }
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<ObjectUid>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Option<ObjectUid>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "an optional object UID")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de::Error;
                let size: u32 = seq
                    .next_element()?
                    .ok_or(Error::missing_field("object UID size"))?;
                match size as usize {
                    ObjectUid::SIZE => {
                        let uid = seq
                            .next_element()?
                            .ok_or(Error::missing_field("object UID"))?;
                        Ok(Some(uid))
                    }
                    0 => Ok(None),
                    size => Err(Error::invalid_length(size, &"object UID size (20)")),
                }
            }
        }
        deserializer.deserialize_tuple(2, Visitor)
    }
}

#[derive(
    derive_more::From,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
)]
pub struct MachineId(String);

#[derive(
    derive_more::From,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
)]
pub struct SessionId(String);
