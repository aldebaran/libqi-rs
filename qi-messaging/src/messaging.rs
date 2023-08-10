use crate::{capabilities, format, message};
pub(crate) use crate::{
    message::Message,
    service::{
        self, CallResult, CallTermination, Error, GetSubject, Reply, RequestId, Service,
        ToRequestId, WithRequestId,
    },
};
pub(crate) mod subject {
    pub(crate) use crate::message::Subject;
}
pub(crate) use subject::Subject;
pub(crate) type Request = service::Request<Call, Notification>;

impl Request {
    pub(crate) fn try_from_message(
        message: Message,
    ) -> Result<Result<Self, Message>, format::Error> {
        let request = match message.kind() {
            message::Kind::Call => Ok(Self::Call(
                Call::new(message.subject()).with_formatted_value(message.into_content()),
            )),
            message::Kind::Post => Ok(Self::Notification(
                Post::new(message.subject())
                    .with_formatted_value(message.into_content())
                    .into(),
            )),
            message::Kind::Event => Ok(Self::Notification(
                Event::new(message.subject())
                    .with_formatted_value(message.into_content())
                    .into(),
            )),
            message::Kind::Cancel => Ok(Self::Notification(
                Cancel::new(message.subject(), message.deserialize_content()?).into(),
            )),
            message::Kind::Capabilities => Ok(Self::Notification(
                Capabilities::new(message.subject(), message.deserialize_content()?).into(),
            )),
            _ => Err(message),
        };
        Ok(request)
    }
}

impl From<Call> for Request {
    fn from(value: Call) -> Self {
        Self::Call(value)
    }
}

impl From<Notification> for Request {
    fn from(value: Notification) -> Self {
        Self::Notification(value)
    }
}

impl From<Post> for Request {
    fn from(value: Post) -> Self {
        Self::Notification(value.into())
    }
}

impl From<Event> for Request {
    fn from(value: Event) -> Self {
        Self::Notification(value.into())
    }
}

impl From<Cancel> for Request {
    fn from(value: Cancel) -> Self {
        Self::Notification(value.into())
    }
}

impl From<Capabilities> for Request {
    fn from(value: Capabilities) -> Self {
        Self::Notification(value.into())
    }
}

pub(crate) type RequestWithId = WithRequestId<Request>;

impl RequestWithId {
    pub(crate) fn try_from_message(
        message: Message,
    ) -> Result<Result<Self, Message>, format::Error> {
        let id = message.id();
        let request = Request::try_from_message(message)?;
        Ok(request.map(|req| Self::new(id, req)))
    }
}

impl TryFrom<RequestWithId> for Message {
    type Error = format::Error;

    fn try_from(value: RequestWithId) -> Result<Self, Self::Error> {
        let id = value.id();
        match value.into_inner() {
            service::Request::Call(call) => Ok(WithRequestId::new(id, call).into()),
            service::Request::Notification(notif) => WithRequestId::new(id, notif).try_into(),
        }
    }
}

pub(crate) type Call = service::Call<Subject>;
pub(crate) type CallWithId = service::CallWithId<Subject>;

impl<S> From<service::CallWithId<S>> for Message
where
    S: Into<Subject> + Clone,
{
    fn from(call: service::CallWithId<S>) -> Self {
        Message::call(call.id(), call.subject().clone().into())
            .set_content(call.into_inner().into_formatted_value())
            .build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub(crate) enum Notification {
    Post(Post),
    Event(Event),
    Cancel(Cancel),
    Capabilities(Capabilities),
}

impl GetSubject for Notification {
    type Subject = Subject;

    fn subject(&self) -> &Self::Subject {
        match self {
            Self::Post(post) => post.subject(),
            Self::Event(event) => event.subject(),
            Self::Cancel(cancel) => cancel.subject(),
            Self::Capabilities(capa) => capa.subject(),
        }
    }
}

impl TryFrom<NotificationWithId> for Message {
    type Error = format::Error;
    fn try_from(notif: NotificationWithId) -> Result<Self, Self::Error> {
        let id = notif.id();
        match notif.into_inner() {
            Notification::Post(post) => Ok(WithRequestId::new(id, post).into()),
            Notification::Event(event) => Ok(WithRequestId::new(id, event).into()),
            Notification::Cancel(cancel) => Ok(WithRequestId::new(id, cancel).into()),
            Notification::Capabilities(capa) => WithRequestId::new(id, capa).try_into(),
        }
    }
}

impl From<Post> for Notification {
    fn from(value: Post) -> Self {
        Notification::Post(value)
    }
}

impl From<Event> for Notification {
    fn from(value: Event) -> Self {
        Notification::Event(value)
    }
}

impl From<Cancel> for Notification {
    fn from(value: Cancel) -> Self {
        Notification::Cancel(value)
    }
}

impl From<Capabilities> for Notification {
    fn from(value: Capabilities) -> Self {
        Notification::Capabilities(value)
    }
}

pub(crate) type NotificationWithId = WithRequestId<Notification>;

impl From<PostWithId> for NotificationWithId {
    fn from(value: PostWithId) -> Self {
        Self::new(value.id(), value.into_inner().into())
    }
}

impl From<EventWithId> for NotificationWithId {
    fn from(value: EventWithId) -> Self {
        Self::new(value.id(), value.into_inner().into())
    }
}

impl From<CancelWithId> for NotificationWithId {
    fn from(value: CancelWithId) -> Self {
        Self::new(value.id(), value.into_inner().into())
    }
}

impl From<CapabilitiesWithId> for NotificationWithId {
    fn from(value: CapabilitiesWithId) -> Self {
        Self::new(value.id(), value.into_inner().into())
    }
}

pub(crate) type Post = service::Post<Subject>;
pub(crate) type PostWithId = service::PostWithId<Subject>;

impl<S> From<service::PostWithId<S>> for Message
where
    S: Into<Subject> + Clone,
{
    fn from(value: service::PostWithId<S>) -> Self {
        Message::post(value.id(), value.subject().clone().into())
            .set_content(value.into_inner().into_formatted_value())
            .build()
    }
}

pub(crate) type Event = service::Event<Subject>;
pub(crate) type EventWithId = service::EventWithId<Subject>;

impl<S> From<service::EventWithId<S>> for Message
where
    S: Into<Subject> + Clone,
{
    fn from(value: service::EventWithId<S>) -> Self {
        Message::event(value.id(), value.subject().clone().into())
            .set_content(value.into_inner().into_formatted_value())
            .build()
    }
}

pub(crate) type Cancel = service::Cancel<Subject>;
pub(crate) type CancelWithId = service::CancelWithId<Subject>;

impl<S> From<service::CancelWithId<S>> for Message
where
    S: Into<Subject> + Clone,
{
    fn from(value: service::CancelWithId<S>) -> Self {
        Message::cancel(
            value.id(),
            value.subject().clone().into(),
            value.inner().call_id(),
        )
        .build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, derive_more::Into)]
pub(crate) struct Capabilities {
    subject: Subject,
    #[into]
    capabilities: capabilities::CapabilitiesMap,
}

impl Capabilities {
    pub(crate) fn new(subject: Subject, capabilities: capabilities::CapabilitiesMap) -> Self {
        Self {
            subject,
            capabilities,
        }
    }
}

pub(crate) type CapabilitiesWithId = WithRequestId<Capabilities>;

impl GetSubject for Capabilities {
    type Subject = Subject;

    fn subject(&self) -> &Self::Subject {
        &self.subject
    }
}

impl TryFrom<CapabilitiesWithId> for Message {
    type Error = format::Error;

    fn try_from(value: CapabilitiesWithId) -> Result<Self, Self::Error> {
        Ok(
            Message::capabilities(value.id(), *value.subject(), &value.inner().capabilities)?
                .build(),
        )
    }
}
