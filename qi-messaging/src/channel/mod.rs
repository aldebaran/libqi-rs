pub(crate) mod request;

use crate::{
    client, format,
    message::{
        self,
        codec::{DecodeError, Decoder, EncodeError, Encoder},
        Subject,
    },
    request::{Id as RequestId, Request, Response},
    server,
};
use futures::{SinkExt, StreamExt};
use request::{Request as ChannelRequest, ResponseFuture};
use std::{
    fmt::Debug,
    sync::atomic::AtomicU32,
    task::{Context, Poll},
};
use tokio::{
    io::{split, AsyncRead, AsyncWrite},
    pin, select,
    sync::mpsc,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{
    codec::{FramedRead, FramedWrite},
    sync::{PollSendError, PollSender},
};
use tower::Service;
use tracing::{debug, debug_span, Instrument};

#[derive(Debug)]
pub(crate) struct Channel {
    client: client::Client,
    current_id: AtomicU32,
}

impl Channel {
    pub(crate) fn new<IO, Svc>(
        io: IO,
        service: Svc,
    ) -> (
        Self,
        impl std::future::Future<Output = Result<(), DispatchError>>,
    )
    where
        IO: AsyncWrite + AsyncRead,
        Svc: tower::Service<Request, Response = Response>,
        Svc::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
    {
        let (input, output) = split(io);
        let mut stream = FramedRead::new(input, Decoder::new());
        let mut sink = FramedWrite::new(output, Encoder);

        const DISPATCH_CHANNEL_SIZE: usize = 1;
        let (client_responses_tx, client_responses_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
        let (client_requests_tx, mut client_requests_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
        let (server_requests_tx, server_requests_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
        let (server_responses_tx, mut server_responses_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);

        let (client, client_dispatch) = client::Client::new(
            ReceiverStream::new(client_responses_rx),
            PollSender::new(client_requests_tx),
        );
        let server = server::serve(
            ReceiverStream::new(server_requests_rx),
            PollSender::new(server_responses_tx),
            service,
        );

        let dispatch = async move {
            let mut stream_is_terminated = false;
            pin!(client_dispatch, server);
            loop {
                select! {
                    message = stream.next(), if !stream_is_terminated => {
                        match message.transpose()? {
                            Some(message) => match Request::try_from_message(message).map_err(DispatchError::MessageIntoRequest)? {
                                Ok(request) => {
                                    // Ignore the result of send, it occurs when the server dropped the request stream, which should mean that the server task has terminated.
                                    let _res = server_requests_tx.send(request).await;
                                }
                                Err(message) => if let Ok((id, response)) = Response::try_from_message(message).map_err(DispatchError::MessageIntoResponse)? {
                                    // Ignore the result of send, it occurs when the client dispatch dropped the response stream, which should mean that the client dispatch task has terminated.
                                    let _res = client_responses_tx.send((id, response)).await;
                                },
                            },
                            None => {
                                debug!("message stream is terminated");
                                stream_is_terminated = true;
                            }
                        }
                    }
                    Some(request) = client_requests_rx.recv() => {
                        let message = request.try_into_message().map_err(DispatchError::RequestIntoMessage)?;
                        sink.send(message).await?;
                    }
                    Some((id, subject, response)) = server_responses_rx.recv() => {
                        if let Some(message) = response.try_into_message(id, subject).map_err(DispatchError::ResponseIntoMessage)? {
                            sink.send(message).await?;
                        }
                    }
                    res = &mut client_dispatch => {
                        res.map_err(DispatchError::ClientDispatch)?;
                        debug!("client dispatch has terminated with success");
                        break Ok(());
                    }
                    res = &mut server => {
                        res?;
                        debug!("server has terminated with success");
                        break Ok(());
                    }
                }
            }
        }.instrument(debug_span!("channel_dispatch"));

        (
            Self {
                client,
                current_id: AtomicU32::new(1),
            },
            dispatch,
        )
    }

    fn make_request_id(&self) -> RequestId {
        let value = self
            .current_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        RequestId::from(value)
    }
}

impl Service<ChannelRequest> for Channel {
    type Response = Response;
    type Error = Error;
    type Future = Future;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.client.poll_ready(cx)
    }
    fn call(&mut self, request: ChannelRequest) -> Self::Future {
        let request_id = self.make_request_id();
        let request = request.into_messaging_request(request_id);
        Future::new(request_id, self.client.call(request))
    }
}

pub(crate) type Future = ResponseFuture<client::Future>;
pub(crate) type Error = client::Error;

#[derive(Debug, thiserror::Error)]
pub(crate) enum DispatchError {
    #[error("messaging decoding error")]
    Decode(#[from] DecodeError),

    #[error("message encoding error")]
    Encode(#[from] EncodeError),

    #[error("client dispatch error")]
    ClientDispatch(#[source] PollSendError<Request>),

    #[error("server error")]
    Server(#[from] PollSendError<(RequestId, Subject, Response)>),

    #[error("error converting a message into a request")]
    MessageIntoRequest(#[source] format::Error),

    #[error("error converting a message into a request")]
    MessageIntoResponse(#[source] message::GetErrorDescriptionError),

    #[error("error converting a request into a message")]
    RequestIntoMessage(#[source] format::Error),

    #[error("error converting a response into a message")]
    ResponseIntoMessage(#[source] format::Error),
}
