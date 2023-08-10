use crate::{
    client, format,
    message::{
        self,
        codec::{DecodeError, Decoder, EncodeError, Encoder},
    },
    messaging::{
        self, CallTermination, CallWithId, NotificationWithId, Reply, RequestWithId, Service,
    },
    server,
};
use futures::{SinkExt, StreamExt};
use std::fmt::Debug;
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
use tracing::trace;

pub(crate) fn open<IO, Svc>(
    io: IO,
    service: Svc,
) -> (
    client::Client,
    impl std::future::Future<Output = Result<(), Error<Svc::CallReply, Svc::Error>>>,
)
where
    IO: AsyncWrite + AsyncRead,
    Svc: Service<CallWithId, NotificationWithId>,
    Svc::Error: ToString + std::fmt::Debug + Send + 'static,
    Svc::CallReply: Into<format::Value> + Send + 'static,
{
    let (input, output) = split(io);
    let mut stream = FramedRead::new(input, Decoder::new()).fuse();
    let mut sink = FramedWrite::new(output, Encoder);

    const DISPATCH_CHANNEL_SIZE: usize = 1;
    let (client_responses_tx, client_responses_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
    let (client_requests_tx, mut client_requests_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
    let (server_requests_tx, server_requests_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
    let (server_responses_tx, mut server_responses_rx) = mpsc::channel(DISPATCH_CHANNEL_SIZE);

    let (client, client_dispatch) = client::setup(
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
                    match RequestWithId::try_from_message(message).map_err(Error::MessageIntoRequest)? {
                        Ok(request) => {
                            let _res = server_requests_tx.send(request).await;
                        }
                        Err(message) => {
                            let id = message.id();
                            let send_response = match message.kind() {
                                message::Kind::Reply => {
                                    let reply = Reply::new(message.into_content());
                                    client_responses_tx.send((id, Ok(reply)))
                                },
                                message::Kind::Canceled => {
                                    client_responses_tx.send((id, Err(CallTermination::Canceled)))
                                },
                                message::Kind::Error => {
                                    let error_description = message.deserialize_error_description().map_err(Error::GetErrorDescription)?;
                                    let error = messaging::Error(error_description);
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
                res = &mut client_dispatch => {
                    res.map_err(Error::ClientDispatch)?;
                    trace!("client dispatch has terminated with success");
                    break Ok(());
                }
                res = &mut server => {
                    res.map_err(Error::Server)?;
                    trace!("server has terminated with success");
                    break Ok(());
                }
            }
        }
    };

    (client, dispatch)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error<SvcRep, SvcErr> {
    #[error("messaging decoding error")]
    Decode(#[from] DecodeError),

    #[error("message encoding error")]
    Encode(#[from] EncodeError),

    #[error("client dispatch error")]
    ClientDispatch(#[source] PollSendError<RequestWithId>),

    #[error("server error")]
    Server(#[source] PollSendError<server::Response<SvcRep, SvcErr>>),

    #[error("error converting a message into a request")]
    MessageIntoRequest(#[source] format::Error),

    #[error("error converting an error message content into an error description")]
    GetErrorDescription(#[source] message::GetErrorDescriptionError),

    #[error("error converting a client request into a message")]
    RequestIntoMessage(#[source] format::Error),

    #[error("error converting as server response into a message")]
    ResponseIntoMessage(#[source] format::Error),
}
