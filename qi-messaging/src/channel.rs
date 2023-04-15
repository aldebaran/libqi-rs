use crate::{
    dispatch, format,
    message::{Id, Recipient},
    types::Dynamic,
    CallResult, Params,
};
use futures::ready;
use pin_project_lite::pin_project;
use std::{future::Future, marker::PhantomData, task::Poll};

#[derive(derive_new::new, Debug)]
pub(crate) struct Channel {
    dispatch: dispatch::OrderSender,
}

impl Channel {
    pub async fn call<T, R>(&self, params: Params<T>) -> Result<Call<R>, RequestStartError>
    where
        T: serde::Serialize,
    {
        let (id, result) = self
            .dispatch
            .call_request(params.recipient, params.argument)
            .await?;
        let canceller = CallCanceller {
            id,
            recipient: params.recipient,
            dispatch: self.dispatch.clone(),
        };
        Ok(Call::new(result, canceller))
    }

    pub async fn post<T>(&self, params: Params<T>) -> Result<(), RequestStartError>
    where
        T: serde::Serialize,
    {
        self.dispatch
            .post(params.recipient, params.argument)
            .await?;
        Ok(())
    }

    pub async fn event<T>(&self, params: Params<T>) -> Result<(), RequestStartError>
    where
        T: serde::Serialize,
    {
        self.dispatch
            .event(params.recipient, params.argument)
            .await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RequestStartError {
    #[error("failed to format call argument into a message payload")]
    PayloadFormat(#[from] format::Error),

    #[error("connection has been dropped")]
    ConnectionDropped,
}

impl From<dispatch::OrderCallError> for RequestStartError {
    fn from(err: dispatch::OrderCallError) -> Self {
        match err {
            dispatch::OrderCallError::Send => Self::ConnectionDropped,
            dispatch::OrderCallError::PayloadFormat(err) => Self::PayloadFormat(err),
        }
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Call<R> {
        #[pin]
        result: dispatch::call::RequestResultReceiver,
        canceller: Option<CallCanceller>,
        phantom: PhantomData<R>,
    }
}

impl<R> Call<R> {
    fn new(result: dispatch::call::RequestResultReceiver, canceller: CallCanceller) -> Self {
        Self {
            result,
            canceller: Some(canceller),
            phantom: PhantomData,
        }
    }

    pub async fn cancel(&mut self) -> Result<(), CancelError> {
        if let Some(canceller) = self.canceller.take() {
            canceller.cancel().await?;
        }
        Ok(())
    }
}

impl<R> Future for Call<R>
where
    R: serde::de::DeserializeOwned,
{
    type Output = Result<CallResult<R>, CallEndError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let resp = ready!(self.project().result.poll(cx))??;
        let res = match resp {
            dispatch::call::Response::Reply(payload) => {
                Ok(CallResult::Ok(format::from_bytes(&payload)?))
            }
            dispatch::call::Response::Error(payload) => {
                let dynamic: Dynamic = format::from_bytes(&payload)?;
                match dynamic.into_string() {
                    Some(err) => Ok(CallResult::Err(err)),
                    None => Err(CallEndError::ResponseErrorDynamicValueIsNotString),
                }
            }
            dispatch::call::Response::Canceled => Ok(CallResult::Canceled),
        };
        Poll::Ready(res)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallEndError {
    #[error("request response is an error with a dynamic value but it does not contain a description string")]
    ResponseErrorDynamicValueIsNotString,

    #[error("format error while deserializing the payload of a request response")]
    ResponsePayloadFormat(#[from] format::Error),

    #[error("message dispatch is closed")]
    MessageDispatchClosed,
}

impl From<dispatch::call::Error> for CallEndError {
    fn from(_err: dispatch::call::Error) -> Self {
        todo!()
    }
}

impl From<dispatch::call::RequestResultRecvError> for CallEndError {
    fn from(_err: dispatch::call::RequestResultRecvError) -> Self {
        Self::MessageDispatchClosed
    }
}

#[derive(Debug)]
struct CallCanceller {
    id: Id,
    recipient: Recipient,
    dispatch: dispatch::OrderSender,
}

impl CallCanceller {
    async fn cancel(self) -> Result<(), CancelError> {
        self.dispatch.call_cancel(self.id, self.recipient).await?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, thiserror::Error)]
pub enum CancelError {
    #[error("connection has been dropped")]
    ConnectionDropped,
}

impl From<dispatch::OrderSendError> for CancelError {
    fn from(err: dispatch::OrderSendError) -> Self {
        let dispatch::OrderSendError = err;
        Self::ConnectionDropped
    }
}
