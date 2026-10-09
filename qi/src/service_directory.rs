use crate::{
    error::Error,
    object::{self, Object},
    service, session,
    value::ServiceId,
};

pub(super) const SERVICE_NAME: &str = "ServiceDirectory";
const SERVICE_ID: ServiceId = ServiceId(1);

// Wire ids are the libqi service-directory contract. Item order below is that order.
// service = { id = 100, text = "get a service (method: service)" },
// services = { id = 101, text = "get all services (method: services)" },
// register_service = { id = 102, text = "register a service (method: registerService)" },
// unregister_service = { id = 103, text = "unregister a service (method: unregisterService)" },
// service_ready = { id = 104, text = "a service is ready (method: serviceReady)" },
// update_service_info = { id = 105, text = "update information of a service (method: updateServiceInfo)"},
// service_added = { id = 106, text = "a service has been added (signal: serviceAdded)" },
// service_removed = { id = 107, text = "a service has been removed (signal: serviceRemoved)" },
// machine_id = { id = 108, text = "get the machine id (method: machineId)" },

// `client` does not emit `impl<T: ServiceDirectory> Object for T`.
// The calculator tests in this crate already emit that blanket impl.
#[qi::object(client)]
pub trait ServiceDirectory {
    #[qi::method]
    async fn service(&self, name: &str) -> Result<service::Info, Error>;

    #[qi::method]
    async fn services(&self) -> Result<Vec<service::Info>, Error>;

    #[qi::method(name = "registerService")]
    async fn register_service(&self, info: &service::Info) -> Result<ServiceId, Error>;

    #[qi::method(name = "unregisterService")]
    async fn unregister_service(&self, id: ServiceId) -> Result<(), Error>;

    #[qi::method(name = "serviceReady")]
    async fn service_ready(&self, id: ServiceId) -> Result<(), Error>;

    #[qi::method(name = "updateServiceInfo")]
    async fn update_service_info(&self, info: &service::Info) -> Result<(), Error>;
}

impl ServiceDirectoryClient {
    pub(super) fn new(session: session::Session) -> Self {
        Self {
            client: object::ObjectClient::new(
                SERVICE_ID,
                service::MAIN_OBJECT_ID,
                object::Uid::default(),
                SERVICE_DIRECTORY_META_OBJECT.clone(),
                session,
            ),
        }
    }
}
