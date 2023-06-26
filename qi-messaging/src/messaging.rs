use crate::{capabilities, format, message};

pub(crate) use crate::{
    message::Message,
    service::{
        self, CallTermination, Error, Reply, RequestId, Service, ToRequestId, ToSubject,
        WithRequestId,
    },
};

pub(crate) mod subject {
    pub(crate) use crate::message::{Action, Object, Service, Subject};
}
pub(crate) use subject::Subject;

pub(crate) type Request = service::Request<Call, Notification>;

impl Request {
    pub(crate) fn try_from_message(
        message: Message,
    ) -> Result<Result<Self, Message>, format::Error> {
        let request = match message.kind() {
            message::Kind::Call => Ok(Self::Call(Call {
                subject: message.subject(),
                payload: message.into_payload(),
            })),
            message::Kind::Post => Ok(Self::Notification(
                Post {
                    subject: message.subject(),
                    payload: message.into_payload(),
                }
                .into(),
            )),
            message::Kind::Event => Ok(Self::Notification(
                Event {
                    subject: message.subject(),
                    payload: message.into_payload(),
                }
                .into(),
            )),
            message::Kind::Cancel => Ok(Self::Notification(
                Cancel {
                    subject: message.subject(),
                    call_id: message.content()?,
                }
                .into(),
            )),
            message::Kind::Capabilities => Ok(Self::Notification(
                Capabilities {
                    subject: message.subject(),
                    capabilities: message.content()?,
                }
                .into(),
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
        match value.inner {
            service::Request::Call(call) => Ok(WithRequestId::new(value.id, call).into()),
            service::Request::Notification(notif) => WithRequestId::new(value.id, notif).try_into(),
        }
    }
}

pub(crate) type Call = service::Call<Subject>;
pub(crate) type CallWithId = service::CallWithId<Subject>;

impl<S> From<service::CallWithId<S>> for Message
where
    S: Into<Subject>,
{
    fn from(call: service::CallWithId<S>) -> Self {
        Message::call(call.id, call.inner.subject.into())
            .set_payload(call.inner.payload)
            .build()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Notification {
    Post(Post),
    Event(Event),
    Cancel(Cancel),
    Capabilities(Capabilities),
}

impl ToSubject for Notification {
    type Subject = Subject;

    fn to_subject(&self) -> Self::Subject {
        match self {
            Self::Post(post) => post.to_subject(),
            Self::Event(event) => event.to_subject(),
            Self::Cancel(cancel) => cancel.to_subject(),
            Self::Capabilities(capa) => capa.to_subject(),
        }
    }
}

impl TryFrom<NotificationWithId> for Message {
    type Error = format::Error;
    fn try_from(notif: NotificationWithId) -> Result<Self, Self::Error> {
        match notif.inner {
            Notification::Post(post) => Ok(WithRequestId::new(notif.id, post).into()),
            Notification::Event(event) => Ok(WithRequestId::new(notif.id, event).into()),
            Notification::Cancel(cancel) => Ok(WithRequestId::new(notif.id, cancel).into()),
            Notification::Capabilities(capa) => WithRequestId::new(notif.id, capa).try_into(),
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
    fn from(WithRequestId { id, inner }: PostWithId) -> Self {
        Self {
            id,
            inner: inner.into(),
        }
    }
}

impl From<EventWithId> for NotificationWithId {
    fn from(WithRequestId { id, inner }: EventWithId) -> Self {
        Self {
            id,
            inner: inner.into(),
        }
    }
}

impl From<CancelWithId> for NotificationWithId {
    fn from(WithRequestId { id, inner }: CancelWithId) -> Self {
        Self {
            id,
            inner: inner.into(),
        }
    }
}

impl From<CapabilitiesWithId> for NotificationWithId {
    fn from(WithRequestId { id, inner }: CapabilitiesWithId) -> Self {
        Self {
            id,
            inner: inner.into(),
        }
    }
}

pub(crate) type Post = service::Post<Subject>;
pub(crate) type PostWithId = service::PostWithId<Subject>;

impl<S> From<service::PostWithId<S>> for Message
where
    S: Into<Subject>,
{
    fn from(value: service::PostWithId<S>) -> Self {
        Message::post(value.id, value.inner.subject.into())
            .set_payload(value.inner.payload)
            .build()
    }
}

pub(crate) type Event = service::Event<Subject>;
pub(crate) type EventWithId = service::EventWithId<Subject>;

impl<S> From<service::EventWithId<S>> for Message
where
    S: Into<Subject>,
{
    fn from(value: service::EventWithId<S>) -> Self {
        Message::event(value.id, value.inner.subject.into())
            .set_payload(value.inner.payload)
            .build()
    }
}

pub(crate) type Cancel = service::Cancel<Subject>;
pub(crate) type CancelWithId = service::CancelWithId<Subject>;

impl<S> From<service::CancelWithId<S>> for Message
where
    S: Into<Subject>,
{
    fn from(value: service::CancelWithId<S>) -> Self {
        Message::cancel(value.id, value.inner.subject.into(), value.inner.call_id).build()
    }
}

#[derive(Debug, Clone, derive_more::Into)]
pub(crate) struct Capabilities {
    pub(crate) subject: Subject,
    #[into]
    pub(crate) capabilities: capabilities::CapabilitiesMap,
}

pub(crate) type CapabilitiesWithId = WithRequestId<Capabilities>;

impl ToSubject for Capabilities {
    type Subject = Subject;

    fn to_subject(&self) -> Self::Subject {
        self.subject
    }
}

impl TryFrom<CapabilitiesWithId> for Message {
    type Error = format::Error;

    fn try_from(value: CapabilitiesWithId) -> Result<Self, Self::Error> {
        Ok(
            Message::capabilities(value.id, value.inner.subject, &value.inner.capabilities)?
                .build(),
        )
    }
}
