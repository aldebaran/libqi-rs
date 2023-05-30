use crate::{
    message::{self},
    request::{Request, Response},
};
use futures::{
    future::{err, BoxFuture},
    FutureExt, Sink, SinkExt, Stream, StreamExt,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    task::{Context, Poll},
};
use tokio::{
    pin, select,
    sync::{mpsc, oneshot},
};
use tokio_util::sync::PollSender;
use tower::Service;
use tracing::{debug, instrument};

type ResponseSender = oneshot::Sender<Response>;

#[derive(Debug)]
pub(crate) struct Client {
    dispatch_sender: PollSender<(Request, ResponseSender)>,
}

impl Client {
    pub(crate) fn new<Si, St>(
        responses_stream: St,
        requests_sink: Si,
    ) -> (Self, impl Future<Output = Result<(), DispatchError>>)
    where
        Si: Sink<Request> + Send + 'static,
        Si::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
        St: Stream<Item = Response> + Send + 'static,
    {
        const DISPATCH_CHANNEL_SIZE: usize = 1;
        let (dispatch_sender, dispatch_receiver) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
        let dispatch_sender = PollSender::new(dispatch_sender);
        let dispatch = dispatch(dispatch_receiver, requests_sink, responses_stream);
        (Self { dispatch_sender }, dispatch)
    }
}

impl Service<Request> for Client {
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.dispatch_sender
            .poll_reserve(cx)
            .map_err(|_err| Error::DispatchIsTerminated)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let (response_sender, response_receiver) = oneshot::channel();
        if let Err(_err) = self.dispatch_sender.send_item((request, response_sender)) {
            return err(Error::DispatchIsTerminated).boxed();
        }
        async move {
            response_receiver
                .await
                .map_err(|_err| Error::DispatchCanceled)
        }
        .boxed()
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("the dispatch task to remote is terminated")]
    DispatchIsTerminated,

    #[error("the dispatch task to remote has canceled the request")]
    DispatchCanceled,
}

#[instrument(level = "debug", skip_all, err)]
async fn dispatch<St, Si>(
    mut request_receiver: mpsc::Receiver<(Request, ResponseSender)>,
    requests_sink: Si,
    responses_stream: St,
) -> Result<(), DispatchError>
where
    Si: Sink<Request>,
    Si::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
    St: Stream<Item = Response>,
{
    let mut ongoing_call_requests = HashMap::new();
    let mut responses_stream_terminated = false;
    let requests_sink = requests_sink.sink_map_err(|err| DispatchError::Sink(err.into()));
    pin!(responses_stream, requests_sink);

    loop {
        select! {
            Some((request, response_sender)) = request_receiver.recv() => {
                if let Request::Call { id, .. } = request {
                    debug!(%id, "registering a call request waiting for a response from the server");
                    ongoing_call_requests.insert(id, response_sender);
                } else {
                    // Other types of requests immediately get their response.
                    if response_sender.send(Response::none()).is_err() {
                        debug!(id = %request.id(), "the client for a call request response has dropped, discarding response");
                    }
                }
                requests_sink.send(request).await?;
            }
            response = responses_stream.next(), if !responses_stream_terminated => {
                match response {
                    Some(response) => if let Some(call_response) = response.as_call_response() {
                        debug!(response = ?response, "received a call response from the server");
                        if let Some(response_sender) = ongoing_call_requests.remove(&call_response.id()) {
                            if let Err(response) = response_sender.send(response) {
                                debug!(response = ?response, "the client for a call request response has dropped, discarding response");
                            }
                        }
                    }
                    None => {
                        debug!("response stream is terminated");
                        responses_stream_terminated = true;
                    },
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

#[derive(Debug, thiserror::Error)]
pub(crate) enum DispatchError {
    #[error("sink error")]
    Sink(#[source] Box<dyn std::error::Error + Sync + Send>),

    #[error("error converting a message into a response")]
    MessageIntoResponse(#[source] message::GetErrorDescriptionError),
}
