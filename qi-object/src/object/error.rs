use qi_value::object::ActionId;

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("canceled")]
    Canceled,

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, thiserror::Error)]
pub enum AnyCallError<E> {
    #[error(transparent)]
    NoSuchMethod(#[from] NoSuchMethodError),

    #[error("error fetching metaobject")]
    MetaObject(#[source] CallError),

    #[error(transparent)]
    Service(E),
}

#[derive(Debug, thiserror::Error)]
pub enum NoSuchMethodError {
    #[error("no such method with id {0}")]
    Id(ActionId),
    #[error("no such method {0}")]
    Name(String),
}

#[derive(Debug, thiserror::Error)]
pub enum NoSuchPropertyError {
    #[error("no such property with id {0}")]
    Id(ActionId),

    #[error("no such property {0}")]
    Name(String),
}

#[derive(Debug, thiserror::Error)]
pub enum NoSuchSignalError {
    #[error("no such signal with id {0}")]
    Id(ActionId),

    #[error("no such signal {0}")]
    Name(String),
}
