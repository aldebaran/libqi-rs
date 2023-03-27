use crate::{call, dispatch, format, types::Dynamic};
use futures::ready;
use pin_project_lite::pin_project;
use std::{future::Future, marker::PhantomData, task::Poll};

#[derive(derive_new::new, Debug)]
pub(crate) struct Channel {
    dispatch_client: dispatch::RequestSender,
}

impl Channel {
    pub fn call<T, R>(&self, params: call::Params<T>) -> Result<Call<R>, CallStartError>
    where
        T: serde::Serialize,
    {
        let req = call_params_to_dispatch_request(params)?;
        let resp_rx = self.dispatch_client.send_call(req)?;
        Ok(Call::new(resp_rx))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallStartError {
    #[error("failed to format call argument into a message payload")]
    PayloadFormat(#[from] format::Error),

    #[error("connection has been dropped")]
    ConnectionDropped,
}

impl From<dispatch::RequestSendError> for CallStartError {
    fn from(_err: dispatch::RequestSendError) -> Self {
        Self::ConnectionDropped
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Call<R> {
        #[pin]
        resp_rx: dispatch::CallResponseReceiver,
        phantom: PhantomData<R>,
    }
}

impl<R> Call<R> {
    pub(crate) fn new(resp_rx: dispatch::CallResponseReceiver) -> Self {
        Self {
            resp_rx,
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
        let response = ready!(self.project().resp_rx.poll(cx))?;
        let result = call_response_to_call_result(response)?;
        Poll::Ready(Ok(result))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallEndError {
    #[error("request response is an error with a dynamic value but it does not contain a description string")]
    ResponseErrorDynamicValueIsNotString,

    #[error("format error while deserializing the payload of a request response")]
    ResponsePayloadFormat(#[from] format::Error),

    #[error("connection has been dropped")]
    ConnectionDropped,
}

impl From<dispatch::ResponseRecvError> for CallEndError {
    fn from(_err: dispatch::ResponseRecvError) -> Self {
        Self::ConnectionDropped
    }
}

fn call_params_to_dispatch_request<T>(
    params: call::Params<T>,
) -> Result<dispatch::CallRequest, CallStartError>
where
    T: serde::Serialize,
{
    Ok(dispatch::CallRequest {
        service: params.service,
        object: params.object,
        action: params.action,
        payload: format::to_bytes(&params.argument)?,
    })
}

fn call_response_to_call_result<T>(
    resp: dispatch::CallResponse,
) -> Result<call::Result<T>, CallEndError>
where
    T: serde::de::DeserializeOwned,
{
    match resp {
        dispatch::CallResponse::Reply(payload) => {
            Ok(call::Result::Ok(format::from_bytes(&payload)?))
        }
        dispatch::CallResponse::Error(payload) => {
            let dynamic: Dynamic = format::from_bytes(&payload)?;
            match dynamic.into_string() {
                Some(err) => Ok(call::Result::Err(err)),
                None => Err(CallEndError::ResponseErrorDynamicValueIsNotString),
            }
        }
        dispatch::CallResponse::Canceled => Ok(call::Result::Canceled),
    }
}
