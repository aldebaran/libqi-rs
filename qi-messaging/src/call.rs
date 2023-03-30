use crate::message::{Action, Object, Recipient, Service};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Params<T = ()> {
    pub recipient: Recipient,
    pub argument: T,
}

impl<T> Params<T> {
    pub fn builder() -> ParamsBuilder<T> {
        ParamsBuilder::default()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ParamsBuilder<T = ()> {
    recipient: Recipient,
    argument: std::marker::PhantomData<T>,
}

impl<T> Default for ParamsBuilder<T> {
    fn default() -> Self {
        Self {
            recipient: Default::default(),
            argument: Default::default(),
        }
    }
}

impl<T> ParamsBuilder<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recipient(mut self, value: Recipient) -> Self {
        self.recipient = value;
        self
    }

    pub fn service(mut self, value: Service) -> Self {
        self.recipient.service = value;
        self
    }

    pub fn object(mut self, value: Object) -> Self {
        self.recipient.object = value;
        self
    }

    pub fn action(mut self, value: Action) -> Self {
        self.recipient.action = value;
        self
    }

    pub fn build(self) -> Params<T>
    where
        T: Default,
    {
        Params {
            recipient: self.recipient,
            argument: T::default(),
        }
    }

    pub fn argument(self, argument: T) -> ParamsBuilderWithArg<T> {
        ParamsBuilderWithArg {
            recipient: self.recipient,
            argument,
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ParamsBuilderWithArg<T = ()> {
    recipient: Recipient,
    argument: T,
}

impl<T> ParamsBuilderWithArg<T> {
    pub fn recipient(mut self, value: Recipient) -> Self {
        self.recipient = value;
        self
    }

    pub fn service(mut self, value: Service) -> Self {
        self.recipient.service = value;
        self
    }

    pub fn object(mut self, value: Object) -> Self {
        self.recipient.object = value;
        self
    }

    pub fn action(mut self, value: Action) -> Self {
        self.recipient.action = value;
        self
    }

    pub fn argument(mut self, argument: T) -> Self {
        self.argument = argument;
        self
    }

    pub fn build(self) -> Params<T> {
        Params {
            recipient: self.recipient,
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
