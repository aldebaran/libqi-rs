use crate::{
    error::Error,
    object::{self, Object},
    service::{self, Info},
    session,
};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use qi_value::{
    object::{MemberAddress, MetaMethod, MetaObject},
    ActionId, IntoValue, Reflect, ServiceId,
};

pub(crate) const SERVICE_NAME: &str = "ServiceDirectory";

#[async_trait]
pub trait ServiceDirectory {
    async fn services(&self) -> Result<Vec<Info>, Error>;
    async fn service(&self, name: &str) -> Result<Info, Error>;
    async fn register_service(&self, info: &Info) -> Result<ServiceId, Error>;
    async fn unregister_service(&self, id: ServiceId) -> Result<(), Error>;
    async fn service_ready(&self, id: ServiceId) -> Result<(), Error>;
    async fn update_service_info(&self, info: &Info) -> Result<(), Error>;
}

#[derive(Clone, Debug)]
pub struct Client {
    object: object::Client,
}

impl Client {
    pub fn new(session: session::Client) -> Self {
        let object = object::Client::new(
            SERVICE_ID,
            service::MAIN_OBJECT_ID,
            object::Uid::default(),
            Meta::get().object.clone(),
            session,
        );
        Self { object }
    }

    pub fn id(&self) -> session::Uid {
        self.object.id()
    }
}

#[async_trait]
impl ServiceDirectory for Client {
    async fn services(&self) -> Result<Vec<Info>, Error> {
        Ok(self
            .object
            .meta_call(MemberAddress::Id(Meta::get().services), ().into_value())
            .await?
            .cast_into()?)
    }

    async fn service(&self, name: &str) -> Result<Info, Error> {
        Ok(self
            .object
            .meta_call(MemberAddress::Id(Meta::get().service), name.into_value())
            .await?
            .cast_into()?)
    }

    async fn register_service(&self, info: &Info) -> Result<ServiceId, Error> {
        Ok(self
            .object
            .meta_call(
                MemberAddress::Id(Meta::get().register_service),
                info.into_value(),
            )
            .await?
            .cast_into()?)
    }

    async fn unregister_service(&self, id: ServiceId) -> Result<(), Error> {
        Ok(self
            .object
            .meta_call(
                MemberAddress::Id(Meta::get().unregister_service),
                id.into_value(),
            )
            .await?
            .cast_into()?)
    }

    async fn service_ready(&self, id: ServiceId) -> Result<(), Error> {
        Ok(self
            .object
            .meta_call(
                MemberAddress::Id(Meta::get().service_ready),
                id.into_value(),
            )
            .await?
            .cast_into()?)
    }

    async fn update_service_info(&self, info: &Info) -> Result<(), Error> {
        Ok(self
            .object
            .meta_call(
                MemberAddress::Id(Meta::get().update_service_info),
                info.into_value(),
            )
            .await?
            .cast_into()?)
    }
}

#[derive(Debug)]
struct Meta {
    object: MetaObject,
    service: ActionId,
    services: ActionId,
    register_service: ActionId,
    unregister_service: ActionId,
    service_ready: ActionId,
    update_service_info: ActionId,
}

impl Meta {
    fn get() -> &'static Self {
        static META: OnceCell<Meta> = OnceCell::new();
        META.get_or_init(|| {
            let service;
            let services;
            let register_service;
            let unregister_service;
            let service_ready;
            let update_service_info;
            let mut method_id = object::ACTION_START_ID;
            let mut builder = MetaObject::builder();
            // Method: service
            builder.add_method({
                service = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(service);
                builder.set_name("service");
                builder.parameter(0).set_type(<&str>::ty());
                builder.return_value().set_type(Info::ty());
                builder.build()
            });
            // Method: services
            builder.add_method({
                services = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(services);
                builder.set_name("services");
                builder.return_value().set_type(Vec::<Info>::ty());
                builder.build()
            });
            // Method: register_service
            builder.add_method({
                register_service = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(register_service);
                builder.set_name("registerService");
                builder.parameter(0).set_type(Info::ty());
                builder.return_value().set_type(ServiceId::ty());
                builder.build()
            });
            // Method: unregister_service
            builder.add_method({
                unregister_service = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(unregister_service);
                builder.set_name("unregisterService");
                builder.parameter(0).set_type(ServiceId::ty());
                builder.build()
            });
            // Method: service_ready
            builder.add_method({
                service_ready = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(service_ready);
                builder.set_name("serviceReady");
                builder.parameter(0).set_type(ServiceId::ty());
                builder.build()
            });
            // Method: update_service_info
            builder.add_method({
                update_service_info = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(update_service_info);
                builder.set_name("updateServiceInfo");
                builder.parameter(0).set_type(Info::ty());
                builder.build()
            });
            let object = builder.build();
            Meta {
                object,
                service,
                services,
                register_service,
                unregister_service,
                service_ready,
                update_service_info,
            }
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

const SERVICE_ID: ServiceId = ServiceId(1);
