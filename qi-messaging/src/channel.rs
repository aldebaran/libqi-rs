use crate::{
    client, format,
    message::{
        self,
        codec::{DecodeError, Decoder, EncodeError, Encoder},
    },
    messaging::{
        CallTermination, CallWithId, IsErrorCanceledTermination, NotificationWithId, RequestId,
        RequestWithId, Service, WithRequestId,
    },
    server,
};
use futures::{SinkExt, StreamExt};
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
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
use tracing::debug;

pub(crate) fn open<IO, Svc>(
    io: IO,
    service: Svc,
) -> (
    client::Client,
    RequestIdSequence,
    impl std::future::Future<Output = Result<(), Error<Svc::Error>>>,
)
where
    IO: AsyncWrite + AsyncRead,
    Svc: Service<CallWithId, NotificationWithId>,
    Svc::Error: IsErrorCanceledTermination + ToString + std::fmt::Debug + Send + 'static,
{
    let (input, output) = split(io);
    let mut stream = FramedRead::new(input, Decoder::new()).fuse();
    let mut sink = FramedWrite::new(output, Encoder);

    const DISPATCH_CHANNEL_SIZE: usize = 1;
    let (client_responses_tx, client_responses_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
    let (client_requests_tx, mut client_requests_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
    let (server_requests_tx, server_requests_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
    let (server_responses_tx, mut server_responses_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);

    let (client_request_sender, client) = client::setup(
        ReceiverStream::new(client_responses_rx),
        PollSender::new(client_requests_tx),
    );
    let server = server::serve(
        ReceiverStream::new(server_requests_rx),
        PollSender::new(server_responses_tx),
        service,
    );

    let dispatch = async move {
        pin!(client, server);
        loop {
            select! {
                Some(message) = stream.next() => {
                    let message = message?;
                    // Ignore the results of send, it occurs when the client or server dropped the
                    // request or response stream, which means that their task have terminated.
                    match RequestWithId::try_from_message(message).map_err(Error::MessageIntoRequest)? {
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
                                    client_responses_tx.send((id, Err(CallTermination::Canceled)))
                                },
                                message::Kind::Error => {
                                    let error = message.error_description().map_err(Error::GetErrorDescription)?;
                                    client_responses_tx.send((id, Err(CallTermination::Error(error))))
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
                    let message = request.try_into().map_err(Error::RequestIntoMessage)?;
                    sink.send(message).await?;
                }
                Some(response) = server_responses_rx.recv() => {
                    let message = response.try_into().map_err(Error::ResponseIntoMessage)?;
                    sink.send(message).await?;
                }
                res = &mut client => {
                    res.map_err(Error::ClientDispatch)?;
                    debug!("client dispatch has terminated with success");
                    break Ok(());
                }
                res = &mut server => {
                    res.map_err(Error::Server)?;
                    debug!("server has terminated with success");
                    break Ok(());
                }
            }
        }
    };

    (client_request_sender, RequestIdSequence::new(), dispatch)
}

#[derive(Debug, Clone)]
pub(crate) struct RequestIdSequence {
    current_id: Arc<AtomicU32>,
}

impl RequestIdSequence {
    fn new() -> Self {
        Self {
            current_id: Arc::new(AtomicU32::new(1)),
        }
    }

    pub(crate) fn pair_with_new_id<T>(&self, request: T) -> WithRequestId<T> {
        let id = self.current_id.fetch_add(1, Ordering::SeqCst);
        WithRequestId {
            id: RequestId::new(id),
            inner: request,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error<SvcErr> {
    #[error("messaging decoding error")]
    Decode(#[from] DecodeError),

    #[error("message encoding error")]
    Encode(#[from] EncodeError),

    #[error("client dispatch error")]
    ClientDispatch(#[source] PollSendError<RequestWithId>),

    #[error("server error")]
    Server(#[source] PollSendError<server::Response<SvcErr>>),

    #[error("error converting a message into a request")]
    MessageIntoRequest(#[source] format::Error),

    #[error("error converting an error message payload into an error description")]
    GetErrorDescription(#[source] message::GetErrorDescriptionError),

    #[error("error converting a client request into a message")]
    RequestIntoMessage(#[source] format::Error),

    #[error("error converting as server response into a message")]
    ResponseIntoMessage(#[source] format::Error),
}
