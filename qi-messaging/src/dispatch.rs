use crate::message::{Flags, Id, Message, Payload, Type};
use futures::{ready, Sink, SinkExt, Stream, StreamExt};
use std::{
    cell::Cell,
    collections::hash_map::{Entry, HashMap},
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use sync::mpsc;
pub use tokio::sync;
use tracing::{instrument, trace, warn};

use self::call::RequestResultSender;

// Arbitrary value to handle backpressure in a request channel.
const CHANNEL_SIZE: usize = 32;

pub(crate) struct Dispatch<Si, St> {
    sink: Pin<Box<Si>>,
    stream: Pin<Box<St>>,
    // TODO: clean shutdown: call close, handle messages in channel, then drop.
    orders: mpsc::Receiver<Order>,
    buffered_message: Option<Message>,
    local_pending_call_requests: HashMap<Id, call::PendingRequest>,
    id_gen: IdGenerator,
}

impl<Si, St, InErr> Dispatch<Si, St>
where
    Si: Sink<Message>,
    St: Stream<Item = Result<Message, InErr>>,
{
    pub(crate) fn new(sink: Si, stream: St) -> (Self, OrderSender) {
        let (order_channel_tx, order_channel_rx) = mpsc::channel(CHANNEL_SIZE);
        (
            Self {
                sink: Box::pin(sink),
                stream: Box::pin(stream),
                orders: order_channel_rx,
                buffered_message: None,
                local_pending_call_requests: HashMap::new(),
                id_gen: IdGenerator::default(),
            },
            OrderSender(order_channel_tx),
        )
    }

    /// Pops every incoming request and send messages.
    fn poll_orders_to_sink(&mut self, cx: &mut Context) -> Poll<Termination<InErr, Si::Error>> {
        // Check if there was a buffered message from a previous call, and retry sending it.
        if let Some(msg) = self.buffered_message.take() {
            if let Err(term) = ready!(self.poll_send_msg(msg, cx)) {
                return Poll::Ready(term);
            }
        }

        let term = loop {
            match self.orders.poll_recv(cx) {
                Poll::Ready(Some(order)) => {
                    if let Err(term) = ready!(self.poll_process_order(order, cx)) {
                        break Some(term);
                    }
                }
                Poll::Ready(None) => break Some(Termination::ClientDropped),
                Poll::Pending => break None,
            }
        };
        if let Some(term) = term {
            return Poll::Ready(term);
        }

        match ready!(self.sink.poll_flush_unpin(cx)) {
            Ok(()) => Poll::Pending,
            Err(err) => Poll::Ready(Termination::OutputError(err)),
        }
    }

    /// Register an order and send the message for it.
    /// Returns `Poll:Pending` if IO is not ready, `Poll::Ready(Ok(())` if the message
    /// was sent successfully, or `Poll::Ready(Err(Termination))` if an error occurred.
    fn poll_process_order(
        &mut self,
        order: Order,
        cx: &mut Context,
    ) -> Poll<Result<(), Termination<InErr, Si::Error>>> {
        debug_assert!(self.buffered_message.is_none());
        match order {
            Order::CallRequest { request, result } => {
                let id = self.id_gen.generate();
                // Check if a request with the same ID already exists.
                match self.local_pending_call_requests.entry(id) {
                    Entry::Occupied(_) => {
                        panic!(
                            "logic error: a pending call request already exists with an id that was newly generated, this might be caused by inconsistent cleanup of old requests"
                        )
                    }
                    Entry::Vacant(entry) => {
                        // Send the request message on the channel.
                        trace!("sending request id={id} {request:?}");
                        let pending_call = call::PendingRequest { result };
                        entry.insert(pending_call);
                        let msg = Message {
                            id,
                            ty: Type::Call,
                            flags: Flags::empty(),
                            recipient: request.recipient,
                            payload: Payload::new(request.payload),
                        };
                        self.poll_send_msg(msg, cx)
                    }
                }
            }
        }
    }

    /// Sends a message.
    /// Returns `Poll:Pending` if IO is not ready, `Poll::Ready(Ok(())` if the message
    /// was sent successfully, or `Poll::Ready(Err(Termination))` if an error occurred.
    fn poll_send_msg(
        &mut self,
        msg: Message,
        cx: &mut Context,
    ) -> Poll<Result<(), Termination<InErr, Si::Error>>> {
        let sent = match self.sink.poll_ready_unpin(cx) {
            Poll::Pending => {
                self.buffered_message = Some(msg);
                return Poll::Pending;
            }
            Poll::Ready(Err(err)) => return Poll::Ready(Err(Termination::OutputError(err))),
            Poll::Ready(Ok(())) => {
                trace!("sending message {msg}");
                match self.sink.start_send_unpin(msg) {
                    Err(err) => Err(Termination::OutputError(err)),
                    Ok(()) => Ok(()),
                }
            }
        };
        Poll::Ready(sent)
    }

    /// Receives messages and resolves pending calls and notifications.
    fn poll_stream_to_orders(&mut self, cx: &mut Context) -> Poll<Termination<InErr, Si::Error>> {
        let term = loop {
            match ready!(self.stream.poll_next_unpin(cx)) {
                Some(Ok(msg)) => self.resolve_orders(msg),
                Some(Err(err)) => break Termination::InputError(err),
                None => break Termination::InputClosed,
            }
        };

        Poll::Ready(term)
    }

    fn resolve_orders(&mut self, msg: Message) {
        enum Class {
            CallResponse(call::Response),
        }
        let class = match msg.ty {
            Type::Reply => Class::CallResponse(call::Response::Reply(msg.payload.into())),
            Type::Error => Class::CallResponse(call::Response::Error(msg.payload.into())),
            Type::Canceled => Class::CallResponse(call::Response::Canceled),
            Type::Call => todo!(),
            Type::Post => todo!(),
            Type::Event => todo!(),
            Type::Capabilities => todo!(),
            Type::Cancel => todo!(),
        };
        let id = msg.id;
        match class {
            Class::CallResponse(resp) => {
                match self.local_pending_call_requests.remove(&id) {
                    Some(call) => call.send_result(Ok(resp)),
                    None => trace!("no local call id={id} found for the response that was received, discarding response"),
                }
            }
        };
    }
}

impl<Si, St> std::fmt::Debug for Dispatch<Si, St> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Debug")
    }
}

impl<Si, St, InErr> Future for Dispatch<Si, St>
where
    Si: Sink<Message>,
    St: Stream<Item = Result<Message, InErr>>,
{
    type Output = Termination<InErr, Si::Error>;

    #[instrument(name = "dispatch", skip_all)]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(term) = self.poll_orders_to_sink(cx) {
            return Poll::Ready(term);
        }
        if let Poll::Ready(term) = self.poll_stream_to_orders(cx) {
            return Poll::Ready(term);
        }
        Poll::Pending
    }
}

#[derive(Debug)]
pub(crate) enum Termination<I, O> {
    ClientDropped,
    InputClosed,
    InputError(I),
    OutputError(O),
}

#[derive(Clone, Debug)]
pub(crate) struct OrderSender(mpsc::Sender<Order>);

impl OrderSender {
    pub(crate) async fn send_call_request(
        &self,
        request: call::Request,
    ) -> Result<call::RequestResultReceiver, OrderSendError> {
        let (result_tx, result_rx) = call::result_channel();
        self.0
            .send(Order::CallRequest {
                request,
                result: result_tx,
            })
            .await
            .map(|()| result_rx)
    }
}

pub(crate) type OrderSendError = mpsc::error::SendError<Order>;

#[derive(Debug)]
pub(crate) enum Order {
    CallRequest {
        request: call::Request,
        result: RequestResultSender,
    },
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct IdGenerator(Cell<Id>);

impl IdGenerator {
    fn generate(&self) -> Id {
        // TODO: use `Cell::update` when available.
        let mut id = self.0.get();
        id.increment();
        self.0.set(id);
        id
    }
}

pub(crate) mod call {
    use crate::message::Recipient;
    use tokio::sync::oneshot;
    use tracing::trace;

    pub(crate) struct Request {
        pub(crate) recipient: Recipient,
        pub(crate) payload: Vec<u8>, // TODO: use types::Value ? it necessitates copying data which is
    }

    impl std::fmt::Debug for Request {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Request")
                .field("recipient", &self.recipient)
                .finish_non_exhaustive()
        }
    }

    #[derive(Debug)]
    pub(super) struct PendingRequest {
        pub(super) result: RequestResultSender,
    }

    impl PendingRequest {
        pub(super) fn send_result(self, resp: RequestResult) {
            if self.result.send(resp).is_err() {
                trace!("call termination receiver dropped, discarding termination demand");
            }
        }
    }

    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub(crate) enum Response {
        Reply(Vec<u8>),
        Error(Vec<u8>),
        Canceled,
    }

    impl std::fmt::Debug for Response {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Reply(_) => f.write_str("Reply"),
                Self::Error(_) => f.write_str("Error"),
                Self::Canceled => f.write_str("Canceled"),
            }
        }
    }

    pub(super) fn result_channel() -> (oneshot::Sender<RequestResult>, RequestResultReceiver) {
        oneshot::channel()
    }

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub(crate) enum Error {}

    pub(crate) type RequestResult = Result<Response, Error>;
    pub(super) type RequestResultSender = oneshot::Sender<RequestResult>;
    pub(crate) type RequestResultReceiver = oneshot::Receiver<RequestResult>;
    pub(crate) type RequestResultRecvError = oneshot::error::RecvError;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Action, Message, Object, Recipient, Service};
    use futures::channel::mpsc;
    use tokio::{spawn, sync::oneshot};

    #[test]
    fn test_message_id_generator() {
        let gen = IdGenerator::default();
        assert_eq!(gen.generate(), Id::new(1));
        assert_eq!(gen.generate(), Id::new(2));
        assert_eq!(gen.generate(), Id::new(3));
    }

    #[test]
    fn test_dispatch_receive_garbage_discarded() {
        todo!()
    }

    #[tokio::test]
    async fn test_dispatch_send_call_receive_response_ok() {
        let (client_sink, mut server_stream) = mpsc::channel(1);
        let (mut server_sink, client_stream) = mpsc::channel::<Result<Message, ()>>(1);

        let (dispatch, dispatch_client) = Dispatch::new(client_sink, client_stream);
        let dispatch = spawn(dispatch);

        let service = Service::new(1);
        let object = Object::new(2);
        let action = Action::new(3);
        let payload = vec![1, 2, 3];

        let mut resp_receiver = dispatch_client
            .send_call_request(call::Request {
                recipient: Recipient {
                    service,
                    object,
                    action,
                },
                payload: payload.clone(),
            })
            .await
            .unwrap();

        // The server must have received the call.
        let call_message = server_stream.next().await.unwrap();
        assert_eq!(
            call_message,
            Message {
                id: Id::from(1),
                ty: Type::Call,
                flags: Flags::empty(),
                recipient: Recipient {
                    service,
                    object,
                    action
                },
                payload: vec![1, 2, 3].into()
            }
        );

        // The response is still awaited.
        assert_eq!(
            resp_receiver.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        );

        // The server sends the reply.
        server_sink
            .send(Ok(Message {
                id: Id::from(1),
                ty: Type::Reply,
                flags: Flags::empty(),
                recipient: Recipient {
                    service,
                    object,
                    action,
                },
                payload: vec![4, 5, 6].into(),
            }))
            .await
            .unwrap();

        // The response arrives.
        let resp = resp_receiver.await.unwrap();
        assert_eq!(resp, Ok(call::Response::Reply(vec![4, 5, 6])));

        drop(dispatch_client);
        dispatch.await.unwrap();
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
    fn test_dispatch_send_call_cancel() {
        todo!()
    }

    #[test]
    fn test_dispatch_send_call_drop_dispatch_causes_error() {
        todo!()
    }
}
