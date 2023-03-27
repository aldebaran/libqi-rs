use crate::{
    codec::{EncodeError as MessageEncodeError, MessageCodec},
    message::{Action, Flags, Id as MessageId, Message, Object, Payload, Service, Type},
};
use futures::{ready, SinkExt, StreamExt};
use std::{
    cell::Cell,
    collections::{hash_map::Entry, HashMap},
    pin::Pin,
    task::Poll,
};
use sync::{mpsc, oneshot};
pub use tokio::sync;
use tokio::{io::AsyncRead, io::AsyncWrite};
use tokio_util::codec::Framed;
use tracing::trace;

#[derive(Debug)]
pub enum Request {
    Call(CallRequest, CallResponseSender),
}

#[derive(Clone, Debug)]
pub struct RequestSender(mpsc::UnboundedSender<Request>);

impl RequestSender {
    pub(crate) fn send_call(
        &self,
        req: CallRequest,
    ) -> Result<CallResponseReceiver, RequestSendError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.0
            .send(Request::Call(req, CallResponseSender(resp_tx)))
            .map(|()| resp_rx)
    }
}

pub type RequestSendError = mpsc::error::SendError<Request>;

pub type RequestReceiver = mpsc::UnboundedReceiver<Request>;

#[derive(Debug)]
pub struct CallResponseSender(oneshot::Sender<CallResponse>);

impl CallResponseSender {
    pub fn is_closed(&self) -> bool {
        self.0.is_closed()
    }

    pub(crate) fn send(self, resp: CallResponse) {
        if self.0.send(resp).is_err() {
            trace!("a response receiver dropped, the response is discarded");
        }
    }
}

pub type CallResponseReceiver = oneshot::Receiver<CallResponse>;

pub type ResponseRecvError = oneshot::error::RecvError;

#[derive(Clone, PartialEq, Eq, PartialOrd, Hash, Debug)]
pub struct CallRequest {
    pub service: Service,
    pub object: Object,
    pub action: Action,
    pub payload: Vec<u8>, // TODO: use ypes::Value ? it necessitates copying data which is
                          // suboptimal.
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Hash, Debug)]
pub enum CallResponse {
    Reply(Vec<u8>),
    Error(Vec<u8>),
    Canceled,
}

#[derive(Debug)]
pub struct Dispatch<IO> {
    io: Framed<Pin<Box<IO>>, MessageCodec>,
    dispatch_req_rx: RequestReceiver,
    dispatch_call_responses_tx: HashMap<MessageId, CallResponseSender>,
    msg_id_gen: MessageIdGenerator,
}

impl<IO> Dispatch<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    pub fn new(io: IO) -> (Self, RequestSender) {
        let (dispatch_req_tx, dispatch_req_rx) = mpsc::unbounded_channel();
        (
            Self {
                io: Framed::new(Box::pin(io), MessageCodec),
                dispatch_req_rx,
                dispatch_call_responses_tx: HashMap::new(),
                msg_id_gen: MessageIdGenerator::default(),
            },
            RequestSender(dispatch_req_tx),
        )
    }

    pub fn poll_dispatch(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), DispatchError>> {
        // "In most cases, if the sink encounters an error, the sink will permanently be unable to
        // receive items.". Therefore, if we encounter any error with the sink, we terminate the
        // connection with an error.
        // Repeat dispatch as long as possible, until one of the operations is pending or over.
        loop {
            if let Err(err) = ready!(self.io.poll_ready_unpin(cx)) {
                // "In most cases, if the sink encounters an error, the sink will permanently be
                // unable to receive items." => terminate the connection with an error.
                return Poll::Ready(Err(err.into()));
            }

            // Handle pending dispatch requests and send messages accordingly.
            loop {
                match self.dispatch_req_rx.poll_recv(cx) {
                    Poll::Ready(Some(dispatch)) => {
                        if let Err(err) = self.register_request(dispatch) {
                            return Poll::Ready(Err(err));
                        }
                        // loop again
                    }
                    Poll::Ready(None) => {
                        trace!("channel was dropped, terminating connection");
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
            if let Poll::Ready(Err(err)) = self.io.poll_flush_unpin(cx) {
                // "In most cases, if the sink encounters an error, the sink will permanently be
                // unable to receive items." => terminate the connection with an error.
                return Poll::Ready(Err(err.into()));
            };

            // Receive messages available in the stream and process any pending request.
            loop {
                match ready!(self.io.poll_next_unpin(cx)) {
                    Some(Ok(msg)) => match MessageClassification::make(msg) {
                        MessageClassification::CallResponse(id, resp) => {
                            match self.dispatch_call_responses_tx.remove(&id) {
                                Some(dispatch_response_tx) => {
                                    trace!("dispatching call response {resp:?}");
                                    dispatch_response_tx.send(resp)
                                }
                                None => {
                                    trace!("discarding unwanted call response {resp:?}");
                                }
                            }
                        }
                    },
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

    fn register_request(&mut self, req: Request) -> Result<(), DispatchError> {
        match req {
            Request::Call(req, resp_tx) => {
                let id = self.msg_id_gen.generate();
                // Check if a request with the same ID already exists.
                match self.dispatch_call_responses_tx.entry(id) {
                    Entry::Occupied(_) => {
                        panic!(
                            "logic error: a request already exists with an id that was newly
                            generated, this might be caused by inconsistent cleanup of old
                            requests, trying a new id"
                        )
                    }
                    Entry::Vacant(entry) => {
                        // Check if since the dispatch request was made, the
                        // request was dropped, and therefore we should abort any
                        // call.
                        if resp_tx.is_closed() {
                            trace!("discarding request for call {req:?} as it was dropped");
                        } else {
                            // Send the request message on the channel.
                            trace!("sending request {req:?}");
                            self.io.start_send_unpin(call_request_to_message(id, req))?;
                            entry.insert(resp_tx);
                        }
                    }
                }
            }
        };
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
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

fn call_request_to_message(id: MessageId, req: CallRequest) -> Message {
    Message {
        id,
        ty: Type::Call,
        flags: Flags::empty(),
        service: req.service,
        object: req.object,
        action: req.action,
        payload: Payload::new(req.payload),
    }
}

enum MessageClassification {
    CallResponse(MessageId, CallResponse),
}

impl MessageClassification {
    fn make(msg: Message) -> Self {
        match msg.ty {
            Type::Reply => Self::CallResponse(msg.id, CallResponse::Reply(msg.payload.into())),
            Type::Error => Self::CallResponse(msg.id, CallResponse::Error(msg.payload.into())),
            Type::Canceled => Self::CallResponse(msg.id, CallResponse::Canceled),
            Type::Call => todo!(),
            Type::Post => todo!(),
            Type::Event => todo!(),
            Type::Capabilities => todo!(),
            Type::Cancel => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{future::poll_fn, SinkExt, StreamExt};
    use tokio::{io::duplex, spawn};

    #[test]
    fn test_message_id_generator() {
        let gen = MessageIdGenerator::default();
        assert_eq!(gen.generate(), MessageId::new(1));
        assert_eq!(gen.generate(), MessageId::new(2));
        assert_eq!(gen.generate(), MessageId::new(3));
    }

    #[tokio::test]
    async fn test_dispatch_send_call_receive_response_ok() {
        let (client, server) = duplex(128); // approximation of an upper bound for messages sizes

        let mut server = Framed::new(server, MessageCodec);

        let (mut dispatch, dispatch_client) = Dispatch::new(client);
        let dispatch_fut = spawn(poll_fn(move |cx| dispatch.poll_dispatch(cx)));

        let service = Service::new(1);
        let object = Object::new(2);
        let action = Action::new(3);
        let payload = vec![1, 2, 3];

        let req = CallRequest {
            service,
            object,
            action,
            payload: payload.clone(),
        };
        let mut resp_receiver = dispatch_client.send_call(req).unwrap();

        // The server must have received the call.
        let call_message = server.next().await.unwrap().unwrap();
        assert_eq!(
            call_message,
            Message {
                id: MessageId::from(1),
                ty: Type::Call,
                flags: Flags::empty(),
                service,
                object,
                action,
                payload: vec![1, 2, 3].into()
            }
        );

        // The response is still awaited.
        assert_eq!(
            resp_receiver.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        );

        // The server sends the reply.
        server
            .send(Message {
                id: MessageId::from(1),
                ty: Type::Reply,
                flags: Flags::empty(),
                service,
                object,
                action,
                payload: vec![4, 5, 6].into(),
            })
            .await
            .unwrap();

        // The response arrives.
        let resp = resp_receiver.await.unwrap();
        assert_eq!(resp, CallResponse::Reply(vec![4, 5, 6]));

        drop(dispatch_client);
        dispatch_fut.await.unwrap();
    }

    #[test]
    fn test_dispatch_send_call_receive_response_error() {
        todo!()
    }

    #[test]
    fn test_dispatch_send_call_receive_response_canceled() {
        todo!()
    }

    #[test]
    fn test_dispatch_send_call_receive_response_bad_recipient() {
        todo!()
    }

    #[test]
    fn test_dispatch_send_call_drop_receiver_sends_cancel() {
        todo!()
    }

    #[test]
    fn test_dispatch_send_call_drop_dispatch_causes_error() {
        todo!()
    }
}
