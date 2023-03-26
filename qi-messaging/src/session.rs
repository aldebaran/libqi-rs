use crate::{
    codec::{EncodeError as MessageEncodeError, MessageCodec},
    dispatch, format,
    message::{Action, Id as MessageId, Object, Service},
    message_types::{
        is_response, CallBuilder, CallBuilderWithArg, CallIntoMessageError, Response,
        ResponseFromMessageError,
    },
};
use futures::{ready, SinkExt, StreamExt};
use pin_project_lite::pin_project;
use std::{
    cell::Cell,
    collections::{hash_map::Entry, HashMap},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::Poll,
};
use tokio::{io::AsyncRead, io::AsyncWrite};
use tokio_util::codec::Framed;
use tracing::trace;

pub trait Session {
    fn call<R>(&self) -> CallRequestBuilder<R>;
}

pub(crate) fn make_bare<IO>(io: IO) -> (Bare, Connection<IO>)
where
    IO: AsyncRead + AsyncWrite,
{
    let (dispatch_req_tx, dispatch_req_rx) = dispatch::request_channel();
    (
        Bare::new(dispatch_req_tx),
        Connection {
            channel: Framed::new(Box::pin(io), MessageCodec),
            dispatch_req_rx,
            dispatch_responses_tx: HashMap::new(),
        },
    )
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Connection<IO> {
    channel: Framed<Pin<Box<IO>>, MessageCodec>,
    dispatch_req_rx: dispatch::RequestReceiver,
    dispatch_responses_tx: HashMap<MessageId, dispatch::ResponseSender>,
}

impl<IO> Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    fn register_dispatch_request(&mut self, req: dispatch::Request) -> Result<(), ConnectionError> {
        match req {
            dispatch::Request::CallRequest { msg, resp_tx } => {
                // Check if a request with the same ID already exists.
                match self.dispatch_responses_tx.entry(msg.id) {
                    Entry::Occupied(_) => {
                        resp_tx.send(Err(dispatch::ResponseError::RequestIdAlreadyExists))
                    }
                    Entry::Vacant(entry) => {
                        // Check if since the dispatch request was made, the
                        // request was dropped, and therefore we should abort any
                        // call.
                        if resp_tx.is_closed() {
                            trace!("discarding request for call {msg} as it was dropped");
                        } else {
                            // Send the request message on the channel.
                            trace!("sending request {msg}");
                            if let Err(err) = self.channel.start_send_unpin(msg) {
                                // "In most cases, if the sink encounters an error, the sink will permanently be
                                // unable to receive items." => terminate the connection with an error.
                                return Err(err.into());
                            }
                            entry.insert(resp_tx);
                        }
                    }
                }
            }
        };
        Ok(())
    }

    fn poll_dispatch(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), ConnectionError>> {
        // Repeat dispatch as long as possible, until one of the operations is pending or over.
        loop {
            if let Err(err) = ready!(self.channel.poll_ready_unpin(cx)) {
                // "In most cases, if the sink encounters an error, the sink will permanently be
                // unable to receive items." => terminate the connection with an error.
                return Poll::Ready(Err(err.into()));
            }

            // Handle pending dispatch requests and send messages accordingly.
            loop {
                match self.dispatch_req_rx.poll_recv(cx) {
                    Poll::Ready(Some(dispatch)) => {
                        if let Err(err) = self.register_dispatch_request(dispatch) {
                            return Poll::Ready(Err(err));
                        }
                        // loop again
                    }
                    Poll::Ready(None) => {
                        trace!("session was dropped, terminating connection");
                        return Poll::Ready(Ok(()));
                    }
                    Poll::Pending => {
                        // No dispatch request at the moment, stop polling for more.
                        break;
                    }
                }
            }

            // TODO: Check if some requests have been canceled and cancel them.

            // Flush messages pending in the sink.
            // It doesn't matter if flushing was still pending or was finished.
            // It will resume on its own when necessary and will be finished once we poll the dispatch
            // again waiting for the sink to be ready.
            if let Poll::Ready(Err(err)) = self.channel.poll_flush_unpin(cx) {
                // "In most cases, if the sink encounters an error, the sink will permanently be
                // unable to receive items." => terminate the connection with an error.
                return Poll::Ready(Err(err.into()));
            };

            // Receive messages available in the stream and process any pending request.
            loop {
                match ready!(self.channel.poll_next_unpin(cx)) {
                    Some(Ok(msg)) => {
                        if is_response(&msg) {
                            if let Some(dispatch_response_tx) =
                                self.dispatch_responses_tx.remove(&msg.id)
                            {
                                trace!("dispatching response {msg}");
                                dispatch_response_tx.send(Ok(msg))
                            }
                        }
                    }
                    Some(Err(err)) => {
                        trace!("discarding message because of a decoding error: \"{err}\"");
                    }
                    None => {
                        trace!("connection stream is closed, terminating connection");
                        return Poll::Ready(Ok(()));
                    }
                }
            }
        }
    }

    fn close(&mut self) {
        todo!()
    }
}

impl<IO> Future for Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    type Output = Result<(), ConnectionError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        self.poll_dispatch(cx)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("messaging encoding error: {0}")]
    MessagingEncoding(#[from] MessageEncodeError),
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct MessageIdGenerator(Cell<MessageId>);

impl MessageIdGenerator {
    fn generate(&self) -> MessageId {
        // TODO: use `Cell::update` when available.
        let mut id = self.0.get();
        id.increment();
        self.0.set(id);
        id
    }
}

#[derive(derive_new::new, Debug)]
pub(crate) struct Bare {
    dispatch_req_tx: dispatch::RequestSender,
    #[new(default)]
    msg_id_gen: MessageIdGenerator,
}

impl Bare {
    fn call_builder(&self) -> CallBuilder {
        CallBuilder::new(self.msg_id_gen.generate())
    }
}

impl Session for Bare {
    fn call<R>(&self) -> CallRequestBuilder<R> {
        CallRequestBuilder {
            dispatch_req_tx: self.dispatch_req_tx.clone(),
            builder: self.call_builder(),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct CallRequestBuilder<R> {
    dispatch_req_tx: dispatch::RequestSender,
    builder: CallBuilder,
    phantom: PhantomData<R>,
}

impl<R> CallRequestBuilder<R> {
    pub fn dynamic_payload(mut self, value: bool) -> Self {
        self.builder = self.builder.dynamic_payload(value);
        self
    }

    pub fn return_type(mut self, value: bool) -> Self {
        self.builder = self.builder.return_type(value);
        self
    }

    pub fn service(mut self, value: Service) -> Self {
        self.builder = self.builder.service(value);
        self
    }

    pub fn object(mut self, value: Object) -> Self {
        self.builder = self.builder.object(value);
        self
    }

    pub fn action(mut self, value: Action) -> Self {
        self.builder = self.builder.action(value);
        self
    }

    pub fn argument<T>(self, argument: T) -> CallRequestBuilderWithArg<T, R> {
        CallRequestBuilderWithArg {
            dispatch_req_tx: self.dispatch_req_tx,
            builder: self.builder.argument(argument),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct CallRequestBuilderWithArg<T, R> {
    dispatch_req_tx: dispatch::RequestSender,
    builder: CallBuilderWithArg<T>,
    phantom: PhantomData<R>,
}

impl<T, R> CallRequestBuilderWithArg<T, R>
where
    T: serde::Serialize,
{
    pub fn send(self) -> Result<SendCallRequest<R>, SendCallRequestError> {
        let call = self.builder.build();
        let message = call.into_message()?;
        let resp_rx = match self.dispatch_req_tx.send_call_request(message) {
            Ok(resp_rx) => resp_rx,
            Err(_) => return Err(SendCallRequestError::BrokenPipe),
        };

        Ok(SendCallRequest {
            resp_rx,
            phantom: PhantomData,
        })
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct SendCallRequest<R> {
        #[pin]
        resp_rx: dispatch::ResponseReceiver,
        phantom: PhantomData<R>,
    }
}

impl<R> SendCallRequest<R> {
    fn handle_dispatch_response(
        response: dispatch::Response,
    ) -> Result<Response<R>, SendCallRequestError>
    where
        R: serde::de::DeserializeOwned,
    {
        match response {
            Ok(msg) => match Response::from_message(msg) {
                Ok(resp) => Ok(resp),
                Err(err) => Err(match err {
                    ResponseFromMessageError::ErrorDynamicValueIsNotString => {
                        SendCallRequestError::ErrorDynamicValueIsNotString
                    }
                    ResponseFromMessageError::PayloadFormat(err) => {
                        SendCallRequestError::PayloadFormat(err)
                    }
                    ResponseFromMessageError::BadType(_) => unreachable!(),
                }),
            },
            Err(dispatch::ResponseError::RequestIdAlreadyExists) => todo!(),
        }
    }
}

impl<R> Future for SendCallRequest<R>
where
    R: serde::de::DeserializeOwned,
{
    type Output = Result<Response<R>, SendCallRequestError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.project()
            .resp_rx
            .poll(cx)
            .map_err(|_| SendCallRequestError::BrokenPipe)?
            .map(Self::handle_dispatch_response)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendCallRequestError {
    #[error("failure to convert call request into a message")]
    IntoMessage(#[from] CallIntoMessageError),

    #[error("request response is an error with a dynamic value but it does not contain a description string")]
    ErrorDynamicValueIsNotString,

    #[error("format error while deserializing the payload of a request response")]
    PayloadFormat(#[from] format::Error),

    #[error(
        "the communication pipe with the request handler or the request dispatch has been broken"
    )]
    BrokenPipe,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_id_generator() {
        let gen = MessageIdGenerator::default();
        assert_eq!(gen.generate(), MessageId::new(1));
        assert_eq!(gen.generate(), MessageId::new(2));
        assert_eq!(gen.generate(), MessageId::new(3));
    }
}
