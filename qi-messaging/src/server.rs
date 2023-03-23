use crate::{
    message::{Action, Object, Service},
    req_rep::CallBuilder,
};

const SERVICE: Service = Service::new(0);
const OBJECT: Object = Object::new(0);
const AUTHENTICATE_ACTION: Action = Action::new(8);

pub trait ToServer {
    fn to_server(self) -> Self;

    fn authenticate(self) -> Self;
}

impl ToServer for CallBuilder {
    fn to_server(self) -> Self {
        self.service(SERVICE).object(OBJECT)
    }

    fn authenticate(self) -> Self {
        self.action(AUTHENTICATE_ACTION)
    }
}
