pub(crate) mod as_service;

use crate::{
    client, format,
    message::{
        self,
        codec::{DecodeError, Decoder, EncodeError, Encoder},
    },
    request::{Id as RequestId, IsCanceledError},
    server,
};
pub(crate) use as_service::{Call, Cancel, Capabilities, Event, Post, Request};
use bytes::Bytes;
use futures::{future, SinkExt, StreamExt};
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
use tracing::debug;

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
        impl std::future::Future<Output = Result<(), DispatchError<Svc::Response, Svc::Error>>>,
    )
    where
        IO: AsyncWrite + AsyncRead,
        Svc: tower::Service<crate::Request>,
        Svc::Response: Into<Option<Bytes>> + std::fmt::Debug + Send + 'static,
        Svc::Error: IsCanceledError + ToString + std::fmt::Debug + Send + 'static,
    {
        let (input, output) = split(io);
        let mut stream = FramedRead::new(input, Decoder::new()).fuse();
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
            pin!(client_dispatch, server);
            loop {
                select! {
                    Some(message) = stream.next() => {
                        let message = message?;
                        // Ignore the results of send, it occurs when the client or server dropped the
                        // request or response stream, which means that their task have terminated.
                        match crate::Request::try_from_message(message).map_err(DispatchError::MessageIntoRequest)? {
                            Ok(request) => {
                                let _res = server_requests_tx.send(request).await;
                            }
                            Err(message) => {
                                let id = message.id();
                                let send_response = match message.kind() {
                                    message::Kind::Reply => {
                                        let reply = message.into_payload();
                                        client_responses_tx.send((id, Ok(reply)))
                                    },
                                    message::Kind::Canceled => {
                                        client_responses_tx.send((id, Err(client::CallError::Canceled)))
                                    },
                                    message::Kind::Error => {
                                        let error = message.error_description().map_err(DispatchError::GetErrorDescription)?;
                                        client_responses_tx.send((id, Err(client::CallError::Error(error))))
                                    },
                                    // Either a message is a request, or it is a call response.
                                    // There are no other cases.
                                    _ => unreachable!(),
                                };
                                let _res = send_response.await;
                            },
                        }
                    }
                    Some(request) = client_requests_rx.recv() => {
                        let message = request.try_into().map_err(DispatchError::RequestIntoMessage)?;
                        sink.send(message).await?;
                    }
                    Some(response) = server_responses_rx.recv() => {
                        let message = response.try_into().map_err(DispatchError::ResponseIntoMessage)?;
                        if let Some(message) = message {
                            sink.send(message).await?;
                        }
                    }
                    res = &mut client_dispatch => {
                        res.map_err(DispatchError::ClientDispatch)?;
                        debug!("client dispatch has terminated with success");
                        break Ok(());
                    }
                    res = &mut server => {
                        res.map_err(DispatchError::Server)?;
                        debug!("server has terminated with success");
                        break Ok(());
                    }
                }
            }
        };

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

    pub(crate) fn ready(&mut self) -> tower::util::Ready<'_, Self, Request> {
        tower::ServiceExt::<Request>::ready(self)
    }
}

macro_rules! impl_service {
    ($($req:ident),+) => {
        $(
            impl Service<as_service::$req> for Channel {
                type Response = <Self::Future as future::TryFuture>::Ok;
                type Error = <Self::Future as future::TryFuture>::Error;
                type Future = as_service::ResponseFuture<<client::Client as Service<crate::$req>>::Future>;

                fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                    <client::Client as Service<crate::$req>>::poll_ready(&mut self.client, cx)
                }

                fn call(&mut self, request: $req) -> Self::Future {
                    let request_id = self.make_request_id();
                    let request = request.into_messaging(request_id);
                    as_service::ResponseFuture::new(request_id, self.client.call(request))
                }
            }
        )+
    };
}

impl_service! {
    Request, Call, Post, Event, Cancel, Capabilities
}

pub(crate) type Error = client::Error;
pub(crate) type ResponseFuture = as_service::ResponseFuture<client::ResponseFuture>;
pub(crate) type CallResponseFuture = as_service::ResponseFuture<client::CallResponseFuture>;
pub(crate) type NoResponseFuture = as_service::ResponseFuture<client::NoResponseFuture>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum DispatchError<SvcResp, SvcErr> {
    #[error("messaging decoding error")]
    Decode(#[from] DecodeError),

    #[error("message encoding error")]
    Encode(#[from] EncodeError),

    #[error("client dispatch error")]
    ClientDispatch(#[source] PollSendError<crate::Request>),

    #[error("server error")]
    Server(#[source] PollSendError<server::Response<SvcResp, SvcErr>>),

    #[error("error converting a message into a request")]
    MessageIntoRequest(#[source] format::Error),

    #[error("error converting an error message payload into an error description")]
    GetErrorDescription(#[source] message::GetErrorDescriptionError),

    #[error("error converting a client request into a message")]
    RequestIntoMessage(#[source] format::Error),

    #[error("error converting as server response into a message")]
    ResponseIntoMessage(#[source] format::Error),
}
