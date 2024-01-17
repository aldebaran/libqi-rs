use crate::{object::BoxObject, service_directory::ServiceDirectory, session, Address, Error};
use async_trait::async_trait;
use qi_value::Value;

#[async_trait]
pub trait Space {
    type ServiceDirectory: ServiceDirectory;

    async fn service(&self, name: &str) -> Result<BoxObject, Error>;

    fn service_directory(&self) -> &Self::ServiceDirectory;
}

#[derive(Default, Clone, Debug)]
pub struct Parameters {
    /// Session references that may be used to connect to the node.
    pub(crate) session_references: Vec<session::Reference>,
    /// Credentials required to authenticate to the node control server.
    pub(crate) credentials: session::authentication::Parameters,
}

impl Parameters {
    pub fn builder() -> ParametersBuilder {
        ParametersBuilder::new()
    }
}

#[derive(Default, Debug)]
pub struct ParametersBuilder(Parameters);

impl ParametersBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_address<A>(mut self, address: A) -> Self
    where
        A: Into<Address>,
    {
        self.0
            .session_references
            .push(session::Reference::Endpoint(address.into()));
        self
    }

    pub fn with_credentials_parameter(mut self, key: String, value: Value<'static>) -> Self {
        self.0.credentials.insert(key, value);
        self
    }

    pub fn build(self) -> Parameters {
        self.0
    }
}
