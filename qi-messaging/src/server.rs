use crate::{
    call::{ParamsBuilder, ParamsBuilderWithArg},
    message::{Action, Object, Service},
};

const SERVICE: Service = Service::new(0);
const OBJECT: Object = Object::new(0);
const AUTHENTICATE_ACTION: Action = Action::new(8);

pub trait ServerCall {
    fn to_server(self) -> Self;

    fn server_authenticate(self) -> Self;
}

impl<T> ServerCall for ParamsBuilder<T> {
    fn to_server(self) -> Self {
        self.service(SERVICE).object(OBJECT)
    }

    fn server_authenticate(self) -> Self {
        self.to_server().action(AUTHENTICATE_ACTION)
    }
}

impl<T> ServerCall for ParamsBuilderWithArg<T> {
    fn to_server(self) -> Self {
        self.service(SERVICE).object(OBJECT)
    }

    fn server_authenticate(self) -> Self {
        self.to_server().action(AUTHENTICATE_ACTION)
    }
}
