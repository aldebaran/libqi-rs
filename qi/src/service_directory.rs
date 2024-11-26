use crate::{
    error::Error,
    messaging,
    object::{self, Object, ObjectExt},
    service, session,
    value::{
        object::{MetaMethod, MetaObject},
        ActionId, Reflect, ServiceId, Value,
    },
};
use async_trait::async_trait;
use once_cell::sync::Lazy;

pub(super) const SERVICE_NAME: &str = "ServiceDirectory";
const SERVICE_ID: ServiceId = ServiceId(1);

#[async_trait]
pub trait ServiceDirectory: Object {
    async fn services(&self) -> Result<Vec<service::Info>, Error>;
    async fn service(&self, name: &str) -> Result<service::Info, Error>;
    async fn register_service(&self, info: &service::Info) -> Result<ServiceId, Error>;
    async fn unregister_service(&self, id: ServiceId) -> Result<(), Error>;
    async fn service_ready(&self, id: ServiceId) -> Result<(), Error>;
    async fn update_service_info(&self, info: &service::Info) -> Result<(), Error>;
}

pub struct Client<Body>(object::Proxy<Body>);

impl<Body> Client<Body> {
    pub(super) fn new(session: session::Session<Body>) -> Self {
        Self(object::Proxy::new(
            SERVICE_ID,
            service::MAIN_OBJECT_ID,
            object::Uid::default(),
            Meta::get().object.clone(),
            session,
        ))
    }
}

impl<Body> Clone for Client<Body> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<Body> std::fmt::Debug for Client<Body> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Client").field(&self.0).finish()
    }
}

#[async_trait]
impl<Body> Object for Client<Body>
where
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    fn meta(&self) -> &MetaObject {
        self.0.meta()
    }

    async fn meta_call(
        &self,
        ident: object::MemberIdent,
        args: Value<'_>,
    ) -> Result<Value<'static>, Error> {
        self.0.meta_call(ident, args).await
    }

    async fn meta_post(&self, ident: object::MemberIdent, value: Value<'_>) {
        self.0.meta_post(ident, value).await
    }

    async fn meta_event(&self, ident: object::MemberIdent, value: Value<'_>) {
        self.0.meta_event(ident, value).await
    }

    fn uid(&self) -> object::Uid {
        self.0.uid()
    }
}

#[async_trait]
impl<Body> ServiceDirectory for Client<Body>
where
    Body: messaging::Body + Send + 'static,
    Body::Error: Send + Sync + 'static,
{
    async fn services(&self) -> Result<Vec<service::Info>, Error> {
        self.0.call(Meta::get().services, ()).await
    }

    async fn service(&self, name: &str) -> Result<service::Info, Error> {
        self.0.call(Meta::get().service, name).await
    }

    async fn register_service(&self, info: &service::Info) -> Result<ServiceId, Error> {
        self.0.call(Meta::get().register_service, info).await
    }

    async fn unregister_service(&self, id: ServiceId) -> Result<(), Error> {
        self.0.call(Meta::get().unregister_service, id).await
    }

    async fn service_ready(&self, id: ServiceId) -> Result<(), Error> {
        self.0.call(Meta::get().service_ready, id).await
    }

    async fn update_service_info(&self, info: &service::Info) -> Result<(), Error> {
        self.0.call(Meta::get().update_service_info, info).await
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
        static META: Lazy<Meta> = Lazy::new(|| {
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
                builder.return_value().set_type(service::Info::ty());
                builder.build()
            });
            // Method: services
            builder.add_method({
                services = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(services);
                builder.set_name("services");
                builder.return_value().set_type(Vec::<service::Info>::ty());
                builder.build()
            });
            // Method: register_service
            builder.add_method({
                register_service = method_id.next().unwrap();
                let mut builder = MetaMethod::builder(register_service);
                builder.set_name("registerService");
                builder.parameter(0).set_type(service::Info::ty());
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
                builder.parameter(0).set_type(service::Info::ty());
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
        });
        &META
    }
}
