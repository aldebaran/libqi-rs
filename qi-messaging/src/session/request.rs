use crate::channel;
pub use crate::message::ServiceSubject as Subject;
use bytes::Bytes;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Request {
    Call(Call),
    Post(Post),
    Event(Event),
}

impl From<Request> for channel::Request {
    fn from(request: Request) -> Self {
        todo!()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Call {
    subject: Subject,
    payload: Bytes,
}

impl From<Call> for channel::Request {
    fn from(call: Call) -> Self {
        todo!()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Post {
    subject: Subject,
    payload: Bytes,
}

impl From<Post> for channel::Request {
    fn from(post: Post) -> Self {
        todo!()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Event {
    subject: Subject,
    payload: Bytes,
}

impl From<Event> for channel::Request {
    fn from(event: Event) -> Self {
        todo!()
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub enum CallError {
    #[error("the call request resulted in an error")]
    Error(String),

    #[error("the call request has been canceled")]
    Canceled,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Response(Bytes);

impl From<channel::Response> for Response {
    fn from(request: channel::Response) -> Self {
        todo!()
    }
}

impl From<Response> for channel::Response {
    fn from(request: Response) -> Self {
        todo!()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CallResponse(Bytes);
