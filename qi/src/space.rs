use crate::{object::BoxObject, service_directory::ServiceDirectory, Error};
use async_trait::async_trait;

#[async_trait]
pub trait Space {
    async fn service(&self, name: &str) -> Result<BoxObject, Error>;

    fn service_directory(&self) -> &dyn ServiceDirectory;
}
