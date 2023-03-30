use crate::{call, dispatch, format, types::Dynamic};
use futures::ready;
use pin_project_lite::pin_project;
use std::{future::Future, marker::PhantomData, task::Poll};

#[derive(derive_new::new, Debug)]
pub(crate) struct Channel {
    dispatch: dispatch::OrderSender,
}

impl Channel {
    pub async fn call<T, R>(&self, params: call::Params<T>) -> Result<Call<R>, CallStartError>
    where
        T: serde::Serialize,
    {
        let req = dispatch::call::Request {
            recipient: params.recipient,
            payload: format::to_bytes(&params.argument)?,
        };
        let result = self.dispatch.send_call_request(req).await?;
        Ok(Call::new(result))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallStartError {
    #[error("failed to format call argument into a message payload")]
    PayloadFormat(#[from] format::Error),

    #[error("connection has been dropped")]
    ConnectionDropped,
}

impl From<dispatch::OrderSendError> for CallStartError {
    fn from(_err: dispatch::OrderSendError) -> Self {
        Self::ConnectionDropped
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Call<R> {
        #[pin]
        result: dispatch::call::RequestResultReceiver,
        phantom: PhantomData<R>,
    }
}

impl<R> Call<R> {
    pub(crate) fn new(result: dispatch::call::RequestResultReceiver) -> Self {
        Self {
            result,
            phantom: PhantomData,
        }
    }
}

impl<R> Future for Call<R>
where
    R: serde::de::DeserializeOwned,
{
    type Output = Result<call::Result<R>, CallEndError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let resp = ready!(self.project().result.poll(cx))??;
        let res = match resp {
            dispatch::call::Response::Reply(payload) => {
                Ok(call::Result::Ok(format::from_bytes(&payload)?))
            }
            dispatch::call::Response::Error(payload) => {
                let dynamic: Dynamic = format::from_bytes(&payload)?;
                match dynamic.into_string() {
                    Some(err) => Ok(call::Result::Err(err)),
                    None => Err(CallEndError::ResponseErrorDynamicValueIsNotString),
                }
            }
            dispatch::call::Response::Canceled => Ok(call::Result::Canceled),
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
