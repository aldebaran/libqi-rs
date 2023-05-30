use crate::{
    client, format,
    message::{self, DecodeError, Decoder, EncodeError, Encoder},
    request::{Request, Response},
    server,
};
use futures::{SinkExt, StreamExt};
use std::future::Future;
use tokio::{
    io::{split, AsyncRead, AsyncWrite},
    pin, select,
    sync::mpsc,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{
    codec::{FramedRead, FramedWrite},
    sync::PollSender,
};
use tower::Service;
use tracing::{debug, debug_span};

#[derive(Debug)]
pub(crate) struct Channel {
    client: client::Client,
}

impl Channel {
    pub(crate) fn new<IO, Svc>(
        io: IO,
        service: Svc,
    ) -> (Self, impl Future<Output = Result<(), DispatchError>>)
    where
        IO: AsyncWrite + AsyncRead,
        Svc: Service<Request, Response = Response>,
        Svc::Error: Into<Box<dyn std::error::Error>>,
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
            let _ = debug_span!("channel_dispatch").entered();
            let mut stream_is_terminated = false;
            pin!(client_dispatch, server);
            loop {
                select! {
                    message = stream.next(), if !stream_is_terminated => {
                        match message.transpose()? {
                            Some(message) => match Request::try_from_message(message).map_err(DispatchError::MessageIntoRequest)? {
                                Ok(request) => {
                                    // Ignore the result of send, it occurs when the server dropped the request stream, which should mean that the server task has terminated.
                                    let _ = server_requests_tx.send(request).await; }
                                Err(message) => if let Ok(response) = Response::try_from_message(message).map_err(DispatchError::MessageIntoResponse)? {
                                    // Ignore the result of send, it occurs when the client dispatch dropped the response stream, which should mean that the client dispatch task has terminated.
                                    let _ = client_responses_tx.send(response).await;
                                },
                            },
                            None => {
                                debug!("message stream is terminated");
                                stream_is_terminated = true;
                            }
                        }
                    }
                    Some(request) = client_requests_rx.recv() => {
                        sink.send(request.into()).await?;
                    }
                    Some(response) = server_responses_rx.recv() => {
                        if let Some(message) = response.try_into().map_err(DispatchError::ResponseIntoMessage)? {
                            sink.send(message).await?;
                        }
                    }
                    res = &mut client_dispatch => {
                        res?;
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
        };

        (Self { client }, dispatch)
    }
}

impl Service<Request> for Channel {
    type Response = Response;
    type Error = <client::Client as Service<Request>>::Error;
    type Future = <client::Client as Service<Request>>::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.client.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        self.client.call(request)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DispatchError {
    #[error("messaging decoding error")]
    Decode(#[from] DecodeError),

    #[error("message encoding error")]
    Encode(#[from] EncodeError),

    #[error("client dispatch error")]
    ClientDispatch(#[from] client::DispatchError),

    #[error("server error")]
    Server(#[from] server::Error),

    #[error("error converting a message into a request")]
    MessageIntoRequest(#[source] format::Error),

    #[error("error converting a message into a request")]
    MessageIntoResponse(#[source] message::GetErrorDescriptionError),

    #[error("error converting a response into a message")]
    ResponseIntoMessage(#[source] format::Error),
}
