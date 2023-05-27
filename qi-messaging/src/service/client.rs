use crate::{
    message::{self, Message},
    request::{Request, Response},
};
use futures::{
    future::{err, ok, BoxFuture},
    FutureExt, Sink, SinkExt, StreamExt, TryStream, TryStreamExt,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    future::{ready, Future},
    task::{Context, Poll},
};
use tokio::{
    pin, select, spawn,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::PollSender;
use tower::Service;
use tracing::{debug, info, instrument};

type ResponseSender = oneshot::Sender<Response>;

#[derive(Debug)]
pub(crate) struct Client {
    dispatch_sender: PollSender<(Request, ResponseSender)>,
    #[allow(unused)]
    dispatch_task: JoinHandle<()>,
}

impl Client {
    pub(crate) fn with_messages_stream_and_sink<Si, St>(
        messages_stream: St,
        messages_sink: Si,
    ) -> Self
    where
        Si: Sink<Message> + Send + 'static,
        Si::Error: Into<Box<dyn std::error::Error + Sync + Send>> + Send,
        St: TryStream<Ok = Message> + Send + 'static,
        St::Error: Into<Box<dyn std::error::Error + Sync + Send>> + Send,
    {
        const DISPATCH_CHANNEL_SIZE: usize = 1;
        let (dispatch_sender, dispatch_receiver) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
        let dispatch_sender = PollSender::new(dispatch_sender);
        let dispatch = dispatch_messages(dispatch_receiver, messages_sink, messages_stream);
        let dispatch = async {
            if let Err(err) = dispatch.await {
                info!(
                    error = &err as &dyn std::error::Error,
                    "dispatch has ended with an error"
                )
            }
        };
        Self {
            dispatch_sender,
            dispatch_task: spawn(dispatch),
        }
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

    #[error("the remote party responded to the call with an malformed error")]
    MalformedCallResponse(#[from] message::GetErrorDescriptionError),
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
    St: TryStream<Ok = Response>,
    St::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
{
    let mut ongoing_call_requests = HashMap::new();
    let responses_stream = responses_stream.map_err(|err| DispatchError::Stream(err.into()));
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
                match response.transpose()? {
                    Some(response) => if let Some(call_response) = response.as_call_response() {
                        debug!(response = ?response, "received a call response from the server");
                        if let Some(response_sender) = ongoing_call_requests.remove(&call_response.id()) {
                            if let Err(response) = response_sender.send(response) {
                                debug!(response = ?response, "the client for a call request response has dropped, discarding response");
                            }
                        }
                    }
                    None => {
                        debug!("responses stream is terminated");
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

fn dispatch_messages<St, Si>(
    request_receiver: mpsc::Receiver<(Request, ResponseSender)>,
    messages_sink: Si,
    messages_stream: St,
) -> impl Future<Output = Result<(), DispatchError>>
where
    Si: Sink<Message>,
    Si::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
    St: TryStream<Ok = Message>,
    St::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
{
    let requests_sink = messages_sink.with(|request: Request| ok::<_, Si::Error>(request.into()));
    let responses_stream = messages_stream.map_err(Into::into).and_then(|message| {
        let response = message
            .try_into()
            .map_err(|err| DispatchError::MessageIntoResponse(err).into());
        ready(response)
    });
    dispatch(request_receiver, requests_sink, responses_stream)
}

#[derive(Debug, thiserror::Error)]
enum DispatchError {
    #[error("sink error")]
    Sink(#[source] Box<dyn std::error::Error + Sync + Send>),

    #[error("stream error")]
    Stream(#[source] Box<dyn std::error::Error + Sync + Send>),

    #[error("error converting a message into a response")]
    MessageIntoResponse(#[source] message::GetErrorDescriptionError),
}
