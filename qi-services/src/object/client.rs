use super::*;
use crate::{
    format,
    messaging::{
        session::{self, Subject},
        CallResult, CallTermination, Service,
    },
    value::object::{ActionId, MetaObject, ObjectId, ObjectUid, ServiceId},
};
use futures::{ready, FutureExt};
use pin_project_lite::pin_project;
use std::{
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::{instrument, trace_span, Instrument};

const SERVICE_MAIN_OBJECT: ObjectId = ObjectId::new(1);

#[derive(Debug, Clone)]
pub struct Client {
    client: session::Client,
    subject_service_object: session::subject::ServiceObject,
    meta_object: MetaObject,
    object_uid: ObjectUid,
}

fn call_action<Args, R>(
    mut client: &session::Client,
    subject_service_object: session::subject::ServiceObject,
    action: ActionId,
    args: Args,
) -> CallFuture<R>
where
    Args: serde::Serialize,
{
    let subject = Subject::new(subject_service_object, action);
    match session::Call::new(subject).with_value(&args) {
        Ok(call) => CallFuture::new_call(client.call(call)),
        Err(err) => CallFuture::new_format_error(err),
    }
}

impl Client {
    #[instrument(level = "trace", ret)]
    pub(crate) async fn connect(
        client: session::Client,
        service_id: ServiceId,
        object_id: ObjectId,
    ) -> CallResult<Self, ConnectError> {
        let subject_service_object = session::subject::ServiceObject::new(service_id, object_id)
            .ok_or(ConnectError::Subject(service_id, object_id))?;

        let meta_object = call_action(
            &client,
            subject_service_object,
            ACTION_ID_METAOBJECT,
            object_id,
        )
        .instrument(trace_span!("get_meta_object"))
        .await
        .map_err(|err| err.map_err(ConnectError::GetServiceDirectoryMetaObject))?;

        Ok(Self {
            client,
            subject_service_object,
            meta_object,
            object_uid: ObjectUid::default(), // TODO: Generate an object UID
        })
    }

    pub(crate) async fn connect_to_service_object(
        client: session::Client,
        service_id: ServiceId,
    ) -> CallResult<Self, ConnectError> {
        Self::connect(client, service_id, SERVICE_MAIN_OBJECT).await
    }

    pub(crate) fn call<Args, R>(&self, name: &str, args: Args) -> CallFuture<R>
    where
        Args: serde::Serialize,
    {
        let method = self
            .meta_object
            .methods
            .iter()
            .find(|(_action, method)| method.name == name);
        let action = match method {
            Some((action, _method)) => *action,
            None => return CallFuture::new_method_not_found(name),
        };
        call_action(&self.client, self.subject_service_object, action, args)
    }

    pub(crate) fn call_action<Args, R>(&self, action: ActionId, args: Args) -> CallFuture<R>
    where
        Args: serde::Serialize,
    {
        if !self.meta_object.methods.contains_key(&action) {
            return CallFuture::new_action_not_found(action);
        }
        call_action(&self.client, self.subject_service_object, action, args)
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing until polled"]
    #[project = CallFutureProj]
    pub enum CallFuture<R> {
        MethodNotFound {
            name: String
        },
        ActionNotFound {
            action: ActionId
        },
        FormatError {
            err: Option<format::Error>
        },
        Call {
            #[pin]
            call: session::CallFuture,
            phantom: PhantomData<R>,
        },
    }
}

impl<R> CallFuture<R> {
    fn new_method_not_found(name: impl Into<String>) -> Self {
        CallFuture::MethodNotFound { name: name.into() }
    }

    fn new_action_not_found(action: impl Into<ActionId>) -> Self {
        CallFuture::ActionNotFound {
            action: action.into(),
        }
    }

    fn new_format_error(err: format::Error) -> Self {
        Self::FormatError { err: Some(err) }
    }

    fn new_call(call: session::CallFuture) -> Self {
        Self::Call {
            call,
            phantom: PhantomData,
        }
    }
}

impl<R> Future for CallFuture<R>
where
    R: serde::de::DeserializeOwned,
{
    type Output = CallResult<R, CallError>;

    #[instrument(level = "trace", skip_all)]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            CallFutureProj::FormatError { err } => match err.take() {
                Some(err) => Poll::Ready(Err(CallTermination::Error(CallError::Format(err)))),
                None => Poll::Pending,
            },
            CallFutureProj::MethodNotFound { name } => Poll::Ready(Err(CallTermination::Error(
                CallError::MethodNotFound(name.clone()),
            ))),
            CallFutureProj::ActionNotFound { action } => Poll::Ready(Err(CallTermination::Error(
                CallError::ActionNotFound(*action),
            ))),
            CallFutureProj::Call { call, .. } => {
                let reply = ready!(call.poll(cx).map_err(|err| err.map_err(CallError::Client)))?;
                let result = reply.value().map_err(CallError::Format)?;
                Poll::Ready(Ok(result))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error(transparent)]
    Client(#[from] session::ClientError),

    #[error("no action with id \"{0}\" was found")]
    ActionNotFound(ActionId),

    #[error("no function named \"{0}\" was found")]
    MethodNotFound(String),

    #[error("format error")]
    Format(#[from] format::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("failure to get the service directory meta object")]
    GetServiceDirectoryMetaObject(#[from] CallError),

    #[error("service subject(service: \"{0}\", object: \"{1}\") is invalid")]
    Subject(ServiceId, ObjectId),
}

const ACTION_ID_REGISTER_EVENT: ActionId = ActionId::new(0);
const ACTION_ID_UNREGISTER_EVENT: ActionId = ActionId::new(1);
const ACTION_ID_METAOBJECT: ActionId = ActionId::new(2);
const ACTION_ID_TERMINATE: ActionId = ActionId::new(3);
const ACTION_ID_PROPERTY: ActionId = ActionId::new(5); // not a typo, there is no action 4
const ACTION_ID_SET_PROPERTY: ActionId = ActionId::new(6);
const ACTION_ID_PROPERTIES: ActionId = ActionId::new(7);
const ACTION_ID_REGISTER_EVENT_WITH_SIGNATURE: ActionId = ActionId::new(8);
const UNRESERVED_ACTION_START_ID: ActionId = ActionId::new(100);

// const ACTION_OBJECT_IS_STATS_ENABLED: ActionId = ActionId::new(80);
// const ACTION_OBJECT_ENABLE_STATS: ActionId = ActionId::new(81);
// const ACTION_OBJECT_STATS: ActionId = ActionId::new(82);
// const ACTION_OBJECT_CLEAR_STATS: ActionId = ActionId::new(83);
// const ACTION_OBJECT_IS_TRACE_ENABLED: ActionId = ActionId::new(84);
// const ACTION_OBJECT_ENABLE_TRACE: ActionId = ActionId::new(85);
// const ACTION_OBJECT_TRACE_OBJECT: ActionId = ActionId::new(86);
