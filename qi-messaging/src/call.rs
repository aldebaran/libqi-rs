use crate::message::{Action, Object, Service};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Params<T = ()> {
    pub service: Service,
    pub object: Object,
    pub action: Action,
    pub argument: T,
}

impl<T> Params<T> {
    pub fn builder() -> ParamsBuilder<T> {
        ParamsBuilder::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ParamsBuilder<T = ()> {
    service: Service,
    object: Object,
    action: Action,
    argument: std::marker::PhantomData<T>,
}

impl<T> Default for ParamsBuilder<T> {
    fn default() -> Self {
        Self {
            service: Default::default(),
            object: Default::default(),
            action: Default::default(),
            argument: Default::default(),
        }
    }
}

impl<T> ParamsBuilder<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn service(mut self, value: Service) -> Self {
        self.service = value;
        self
    }

    pub fn object(mut self, value: Object) -> Self {
        self.object = value;
        self
    }

    pub fn action(mut self, value: Action) -> Self {
        self.action = value;
        self
    }

    pub fn argument(self, argument: T) -> ParamsBuilderWithArg<T> {
        ParamsBuilderWithArg {
            service: self.service,
            object: self.object,
            action: self.action,
            argument,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ParamsBuilderWithArg<T = ()> {
    service: Service,
    object: Object,
    action: Action,
    argument: T,
}

impl<T> ParamsBuilderWithArg<T> {
    pub fn service(mut self, value: Service) -> Self {
        self.service = value;
        self
    }

    pub fn object(mut self, value: Object) -> Self {
        self.object = value;
        self
    }

    pub fn action(mut self, value: Action) -> Self {
        self.action = value;
        self
    }

    pub fn argument(mut self, argument: T) -> Self {
        self.argument = argument;
        self
    }

    pub fn build(self) -> Params<T> {
        Params {
            service: self.service,
            object: self.object,
            action: self.action,
            argument: self.argument,
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Result<T = ()> {
    Ok(T),
    Err(String),
    Canceled,
}
