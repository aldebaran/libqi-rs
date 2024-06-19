use crate::{service_directory::ServiceDirectory, Object, Result};
use std::future::Future;

pub trait Space {
    fn service(&self, name: &str) -> impl Future<Output = Result<impl Object>> + Send;

    fn service_directory(&self) -> &dyn ServiceDirectory;
}
