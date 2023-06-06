use crate::{
    message::Id,
    request::{Call, Cancel, Capabilities, Event, Post, Request},
};
use bytes::Bytes;
use futures::{future, ready, FutureExt, Sink, SinkExt, Stream, StreamExt};
use std::{
    collections::HashMap,
    fmt::Debug,
    task::{Context, Poll},
};
use tokio::{
    pin, select,
    sync::{mpsc, oneshot},
};
use tokio_util::sync::PollSender;
use tracing::{debug, instrument};

#[derive(Debug)]
pub(crate) struct Client {
    dispatch_sender: PollSender<DispatchRequest>,
}

impl Client {
    pub(crate) fn new<Si, St>(
        responses_stream: St,
        requests_sink: Si,
    ) -> (
        Self,
        impl std::future::Future<Output = Result<(), Si::Error>>,
    )
    where
        Si: Sink<Request>,
        Si::Error: std::error::Error,
        St: Stream<Item = (Id, Result<Bytes, CallError>)>,
    {
        const DISPATCH_CHANNEL_SIZE: usize = 1;
        let (dispatch_sender, dispatch_receiver) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
        let dispatch_sender = PollSender::new(dispatch_sender);
        let dispatch = dispatch(dispatch_receiver, requests_sink, responses_stream);
        (Self { dispatch_sender }, dispatch)
    }

    pub(crate) fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.dispatch_sender
            .poll_reserve(cx)
            .map_err(|_err| DispatchError::DispatchIsTerminated.into())
    }
}

impl tower::Service<Request> for Client {
    type Response = Option<Bytes>;
    type Error = Error;
    type Future = Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        match request {
            Request::Call(call) => self.call(call).into(),
            Request::Post(post) => self.call(post).into(),
            Request::Event(event) => self.call(event).into(),
            Request::Cancel(cancel) => self.call(cancel).into(),
            Request::Capabilities(capabilities) => self.call(capabilities).into(),
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub(crate) enum Future {
    Call(CallFuture),
    None,
    Error(Option<Error>),
}

impl std::future::Future for Future {
    type Output = Result<Option<Bytes>, Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            Self::Call(call) => call.poll_unpin(cx).map_ok(Some),
            Self::None => Poll::Ready(Ok(None)),
            Self::Error(err) => match err.take() {
                Some(err) => Poll::Ready(Err(err)),
                None => Poll::Pending,
            },
        }
    }
}

impl From<CallFuture> for Future {
    fn from(future: CallFuture) -> Self {
        match future {
            CallFuture::WaitForResponse(..) => Self::Call(future),
            CallFuture::Error(err) => Self::Error(err),
        }
    }
}

impl From<future::Ready<Result<(), Error>>> for Future {
    fn from(future: future::Ready<Result<(), Error>>) -> Self {
        match future.into_inner() {
            Ok(()) => Future::None,
            Err(err) => Future::Error(Some(err)),
        }
    }
}

impl tower::Service<Call> for Client {
    type Response = Bytes;
    type Error = Error;
    type Future = CallFuture;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.dispatch_sender
            .poll_reserve(cx)
            .map_err(|_err| DispatchError::DispatchIsTerminated.into())
    }

    fn call(&mut self, request: Call) -> Self::Future {
        let (response_sender, response_receiver) = oneshot::channel();
        match self.dispatch_sender.send_item(DispatchRequest::Call {
            request,
            response_sender,
        }) {
            Ok(()) => CallFuture::WaitForResponse(response_receiver),
            Err(_send_err) => CallFuture::Error(Some(DispatchError::DispatchIsTerminated.into())),
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub(crate) enum CallFuture {
    WaitForResponse(oneshot::Receiver<Result<Bytes, CallError>>),
    Error(Option<Error>),
}

impl std::future::Future for CallFuture {
    type Output = Result<Bytes, Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            CallFuture::WaitForResponse(response_receiver) => {
                let res = match ready!(response_receiver.poll_unpin(cx)) {
                    Ok(Ok(reply)) => Ok(reply),
                    Ok(Err(req_err)) => Err(req_err.into()),
                    Err(_recv_err) => Err(DispatchError::DispatchCanceled.into()),
                };
                Poll::Ready(res)
            }
            CallFuture::Error(err) => match err.take() {
                Some(err) => Poll::Ready(Err(err)),
                None => Poll::Pending,
            },
        }
    }
}

macro_rules! impl_service_for_no_response_requests {
    ($($req:ty),+) => {
        $(
            impl tower::Service<$req> for Client {
                type Response = ();
                type Error = Error;
                type Future = future::Ready<Result<(), Error>>;

                fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                    self.dispatch_sender
                        .poll_reserve(cx)
                        .map_err(|_err| DispatchError::DispatchIsTerminated.into())
                }

                fn call(&mut self, request: $req) -> Self::Future {
                    let res = self
                        .dispatch_sender
                        .send_item(DispatchRequest::Other(request.into()))
                        .map_err(|_send_err| DispatchError::DispatchIsTerminated.into());
                    future::ready(res)
                }
            }
        )+
    };
}

impl_service_for_no_response_requests! {
    Post, Event, Cancel, Capabilities
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("dispatch error")]
    Dispatch(#[from] DispatchError),

    #[error(transparent)]
    Call(#[from] CallError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CallError {
    #[error("request has been canceled")]
    Canceled,

    #[error("{0}")]
    Error(String),
}

#[instrument(level = "debug", skip_all, err)]
async fn dispatch<St, Si>(
    mut request_receiver: mpsc::Receiver<DispatchRequest>,
    requests_sink: Si,
    responses_stream: St,
) -> Result<(), Si::Error>
where
    Si: Sink<Request>,
    Si::Error: std::error::Error,
    St: Stream<Item = (Id, Result<Bytes, CallError>)>,
{
    let mut ongoing_call_requests = HashMap::new();
    let requests_sink = requests_sink;
    let responses_stream = responses_stream.fuse();
    pin!(responses_stream, requests_sink);

    loop {
        select! {
            Some(request) = request_receiver.recv() => {
                let request = match request {
                    DispatchRequest::Call {
                        request,
                        response_sender,
                    } => {
                        let id = request.id;
                        debug!(%id, "registering a call request waiting for a response from the server");
                        ongoing_call_requests.insert(id, response_sender);
                        request.into()
                    }
                    DispatchRequest::Other(request) => request,
                };
                requests_sink.send(request).await?;
            }
            Some((id, response)) = responses_stream.next() => {
                debug!(response = ?response, "received a call response from the server");
                if let Some(response_sender) = ongoing_call_requests.remove(&id) {
                    if let Err(response) = response_sender.send(response) {
                        debug!(response = ?response, "the client for a call request response has dropped, discarding response");
                    }
                }
            }
            else => {
                debug!("client dispatch is finished");
                break Ok(());
            }
        }

        // Cleanup ongoing call requests for which the client has dropped the channel.
        ongoing_call_requests.retain(|_id, response_sender| !response_sender.is_closed())
    }
}

#[derive(Debug)]
enum DispatchRequest {
    Call {
        request: Call,
        response_sender: oneshot::Sender<Result<Bytes, CallError>>,
    },
    Other(Request),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DispatchError {
    #[error("the dispatch task to remote is terminated")]
    DispatchIsTerminated,

    #[error("the dispatch task to remote has canceled the request")]
    DispatchCanceled,
}
