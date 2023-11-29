use crate::{
    object,
    service::{self, ServiceInfo},
    session, Error, Node, Object, ServiceDirectory,
};
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct Client<'s> {
    access_node: &'s Node,
    sd_client: session::Client,
}

impl<'s> Client<'s> {
    pub(crate) fn new(access_node: &'s Node, sd_client: session::Client) -> Self {
        Self {
            access_node,
            sd_client,
        }
    }

    pub async fn service(&self, name: &str) -> Result<Box<dyn Object + Send + Sync>, Error> {
        let service = self.service_info(name).await?;
        let endpoints = self.access_node.sort_endpoints(&service);
        let session = self
            .access_node
            .session_to_service_node(endpoints, service.name, None)
            .await?;
        let object = object::Client::new(service.service_id, service::MAIN_OBJECT_ID, session);
        Ok(Box::new(object))
    }

    pub fn service_directory(&self) -> &session::Client {
        &self.sd_client
    }
}

#[async_trait]
impl<'s> ServiceDirectory for Client<'s> {
    async fn service_info(&self, name: &str) -> Result<ServiceInfo, Error> {
        Ok(self.sd_client.service_info(name).await?)
    }
}
