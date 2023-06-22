use crate::{capabilities, format, message};

pub(crate) use crate::{
    message::Message,
    service::{
        self, CallTermination, IsErrorCanceledTermination, RequestId, Service, ToRequestId,
        ToSubject, TryIntoMessageWithId, WithRequestId,
    },
};

pub(crate) mod subject {
    pub(crate) use crate::message::{Action, Object, Service, Subject};
}
use sealed::sealed;
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

pub(crate) type Call = service::Call<Subject>;

impl Call {
    pub(crate) fn into_message(self, id: message::Id) -> Message {
        Message::call(id, self.subject)
            .set_content_bytes(self.payload)
            .build()
    }
}

impl<S> TryIntoMessageWithId for service::Call<S>
where
    S: Into<Subject>,
{
    fn try_into_message(self, id: message::Id) -> Result<Message, format::Error> {
        Ok(Call {
            subject: self.subject.into(),
            payload: self.payload,
        }
        .into_message(id))
    }
}

pub(crate) type CallWithId = service::CallWithId<Subject>;

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

impl TryIntoMessageWithId for Notification {
    fn try_into_message(self, id: message::Id) -> Result<Message, format::Error> {
        match self {
            Self::Post(post) => post.try_into_message(id),
            Self::Event(event) => event.try_into_message(id),
            Self::Cancel(cancel) => cancel.try_into_message(id),
            Self::Capabilities(capa) => capa.try_into_message(id),
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

impl Post {
    pub(crate) fn into_message(self, id: message::Id) -> Message {
        Message::post(id, self.subject)
            .set_content_bytes(self.payload)
            .build()
    }
}

impl<S> TryIntoMessageWithId for service::Post<S>
where
    S: Into<Subject>,
{
    fn try_into_message(self, id: message::Id) -> Result<Message, format::Error> {
        Ok(Post {
            subject: self.subject.into(),
            payload: self.payload,
        }
        .into_message(id))
    }
}

pub(crate) type PostWithId = WithRequestId<Post>;

pub(crate) type Event = service::Event<Subject>;

impl Event {
    pub(crate) fn into_message(self, id: message::Id) -> Message {
        Message::event(id, self.subject)
            .set_content_bytes(self.payload)
            .build()
    }
}

impl<S> TryIntoMessageWithId for service::Event<S>
where
    S: Into<Subject>,
{
    fn try_into_message(self, id: message::Id) -> Result<Message, format::Error> {
        Ok(Event {
            subject: self.subject.into(),
            payload: self.payload,
        }
        .into_message(id))
    }
}

pub(crate) type EventWithId = WithRequestId<Event>;

pub(crate) type Cancel = service::Cancel<Subject>;

impl Cancel {
    pub(crate) fn into_message(self, id: message::Id) -> Message {
        Message::cancel(id, self.subject, self.call_id).build()
    }
}

impl<S> TryIntoMessageWithId for service::Cancel<S>
where
    S: Into<Subject>,
{
    fn try_into_message(self, id: message::Id) -> Result<Message, format::Error> {
        Ok(Cancel {
            subject: self.subject.into(),
            call_id: self.call_id,
        }
        .into_message(id))
    }
}

pub(crate) type CancelWithId = WithRequestId<Cancel>;

#[derive(Debug, Clone, derive_more::Into)]
pub(crate) struct Capabilities {
    pub(crate) subject: Subject,
    #[into]
    pub(crate) capabilities: capabilities::CapabilitiesMap,
}

impl ToSubject for Capabilities {
    type Subject = Subject;

    fn to_subject(&self) -> Self::Subject {
        self.subject
    }
}

impl TryIntoMessageWithId for Capabilities {
    fn try_into_message(self, id: message::Id) -> Result<Message, format::Error> {
        Ok(Message::capabilities(id, self.subject, &self.capabilities)?.build())
    }
}

pub(crate) type CapabilitiesWithId = WithRequestId<Capabilities>;

#[sealed]
pub(crate) trait TryIntoFailureMessage {
    fn try_into_failure_message(
        self,
        id: RequestId,
        subject: Subject,
    ) -> Result<Message, format::Error>;
}

#[sealed]
impl<T> TryIntoFailureMessage for T
where
    T: IsErrorCanceledTermination + ToString,
{
    fn try_into_failure_message(
        self,
        id: RequestId,
        subject: Subject,
    ) -> Result<Message, format::Error> {
        Ok(if self.is_canceled() {
            Message::canceled(id, subject).build()
        } else {
            Message::error(id, subject, &self.to_string())?.build()
        })
    }
}
