use crate::{
    object,
    service::{self, ServiceInfo},
    session, Error,
};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use qi_messaging::message;
use qi_value::{
    object::{MetaMethod, MetaObject},
    ActionId, ServiceId, Type,
};

pub(crate) const SERVICE_NAME: &str = "ServiceDirectory";

#[async_trait]
pub trait ServiceDirectory {
    async fn service_info(&self, name: &str) -> Result<ServiceInfo, Error>;
}

#[async_trait]
impl ServiceDirectory for session::Client {
    async fn service_info(&self, name: &str) -> Result<ServiceInfo, Error> {
        let address = message::Address::new(
            SERVICE_ID,
            service::MAIN_OBJECT_ID,
            Meta::get().methods.service_info,
        );
        Ok(self.call_into_value(address, name).await?)
    }
}

#[derive(Debug)]
struct Meta {
    object: MetaObject,
    methods: Methods,
}

impl Meta {
    fn get() -> &'static Self {
        static META: OnceCell<Meta> = OnceCell::new();
        META.get_or_init(|| {
            let mut methods = Methods::default();
            let mut method_id = object::ACTION_START_ID;
            let mut builder = MetaObject::builder();
            builder.add_method({
                methods.service_info = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(methods.service_info);
                builder.set_name("add");
                builder.parameter(0).set_type(Type::Int32);
                builder.return_value().set_type(Type::Int32);
                builder.build()
            });
            let object = builder.build();
            Meta { object, methods }
            // service = { id = 100, text = "get a service (method: service)" },
            // services = { id = 101, text = "get all services (method: services)" },
            // register_service = { id = 102, text = "register a service (method: registerService)" },
            // unregister_service = { id = 103, text = "unregister a service (method: unregisterService)" },
            // service_ready = { id = 104, text = "a service is ready (method: serviceReady)" },
            // update_service_info = { id = 105, text = "update information of a service (method: updateServiceInfo)"},
            // service_added = { id = 106, text = "a service has been added (signal: serviceAdded)" },
            // service_removed = { id = 107, text = "a service has been removed (signal: serviceRemoved)" },
            // machine_id = { id = 108, text = "get the machine id (method: machineId)" },
        })
    }
}

#[derive(Default, Debug)]
struct Methods {
    service_info: ActionId,
}

const SERVICE_ID: ServiceId = ServiceId(1);
