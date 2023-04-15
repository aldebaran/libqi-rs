use crate::{
    format,
    message::{Flags, Id, Message, Payload, Recipient, Type},
};
use futures::{Sink, Stream};
use std::{
    collections::hash_map::{Entry, HashMap},
    pin::Pin,
    sync::{atomic::AtomicU32, Arc},
    task::{Context, Poll},
};
use sync::mpsc;
pub use tokio::sync;
use tracing::trace;

// Arbitrary value to handle backpressure in a request channel.
const CHANNEL_SIZE: usize = 32;

pub(crate) struct Dispatch {
    orders: mpsc::Receiver<Order>,
    local_pending_call_requests: HashMap<(Id, Recipient), call::PendingRequest>,
    id_gen: IdGenerator,
}

impl Dispatch {
    pub(crate) fn new() -> (Self, OrderSender) {
        let (orders_tx, orders_rx) = mpsc::channel(CHANNEL_SIZE);
        let id_gen = IdGenerator::default();
        (
            Self {
                orders: orders_rx,
                local_pending_call_requests: HashMap::new(),
                id_gen: id_gen.clone(),
            },
            OrderSender {
                sender: orders_tx,
                id_gen,
            },
        )
    }

    fn cleanup_pending_requests(&mut self) {
        self.local_pending_call_requests
            .retain(|_, call| !call.result.is_closed());
    }

    fn poll_order_message(&mut self, cx: &mut Context) -> Poll<Option<Message>> {
        self.orders
            .poll_recv(cx)
            .map(|order| order.map(|order| self.order_to_message(order)))
    }

    fn order_to_message(&mut self, order: Order) -> Message {
        match order {
            Order::CallRequest(request) => {
                // Check if a request with the same ID already exists.
                match self
                    .local_pending_call_requests
                    .entry((request.id, request.recipient))
                {
                    Entry::Occupied(_) => {
                        panic!(
                            "logic error: a pending call request already exists with an id that was newly generated, this might be caused by inconsistent cleanup of old requests"
                        )
                    }
                    Entry::Vacant(entry) => {
                        // Send the request message on the channel.
                        let pending_call = call::PendingRequest {
                            result: request.result,
                        };
                        entry.insert(pending_call);
                        Message {
                            id: request.id,
                            ty: Type::Call,
                            flags: Flags::empty(),
                            recipient: request.recipient,
                            payload: Payload::new(request.payload),
                        }
                    }
                }
            }
            Order::CallCancel(cancel) => Message {
                id: self.id_gen.generate(),
                ty: Type::Cancel,
                flags: Flags::empty(),
                recipient: cancel.recipient,
                payload: Payload::new(
                    format::to_bytes(&cancel.id).expect("failed to serialize a message ID"),
                ),
            },
            Order::Post(post) => Message {
                id: self.id_gen.generate(),
                ty: Type::Post,
                flags: Flags::empty(),
                recipient: post.recipient,
                payload: Payload::new(post.payload),
            },
            Order::Event(event) => Message {
                id: self.id_gen.generate(),
                ty: Type::Event,
                flags: Flags::empty(),
                recipient: event.recipient,
                payload: Payload::new(event.payload),
            },
        }
    }

    fn process_message(&mut self, msg: Message) {
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
            Class::CallResponse(resp) => match self
                .local_pending_call_requests
                .remove(&(id, msg.recipient))
            {
                Some(call) => call.send_result(Ok(resp)),
                None => {
                    trace!(%id, "no local call found for the response that was received, discarding response")
                }
            },
        };
    }
}

impl std::fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Debug")
    }
}

impl Stream for Dispatch {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.cleanup_pending_requests();
        self.poll_order_message(cx)
    }
}

impl Sink<Message> for Dispatch {
    type Error = Infallible;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.process_message(item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // TODO
        Poll::Ready(Ok(()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub(crate) enum Infallible {}

#[derive(Clone, Debug)]
pub(crate) struct OrderSender {
    id_gen: IdGenerator,
    sender: mpsc::Sender<Order>,
}

impl OrderSender {
    pub(crate) async fn call_request<T>(
        &self,
        recipient: Recipient,
        argument: T,
    ) -> Result<(Id, call::RequestResultReceiver), OrderCallError>
    where
        T: serde::Serialize,
    {
        let id = self.id_gen.generate();
        let (result_tx, result_rx) = call::result_channel();
        self.sender
            .send(Order::CallRequest(call::Request {
                id,
                recipient,
                payload: format::to_bytes(&argument)?,
                result: result_tx,
            }))
            .await?;
        Ok((id, result_rx))
    }

    pub(crate) async fn call_cancel(
        &self,
        id: Id,
        recipient: Recipient,
    ) -> Result<(), OrderSendError> {
        self.sender
            .send(Order::CallCancel(call::Cancel { id, recipient }))
            .await?;
        Ok(())
    }

    pub(crate) async fn post<T>(
        &self,
        recipient: Recipient,
        argument: T,
    ) -> Result<(), OrderPostError>
    where
        T: serde::Serialize,
    {
        self.sender
            .send(Order::Post(Post {
                recipient,
                payload: format::to_bytes(&argument)?,
            }))
            .await?;
        Ok(())
    }

    pub(crate) async fn event<T>(
        &self,
        recipient: Recipient,
        argument: T,
    ) -> Result<(), OrderPostError>
    where
        T: serde::Serialize,
    {
        self.sender
            .send(Order::Event(Event {
                recipient,
                payload: format::to_bytes(&argument)?,
            }))
            .await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum OrderCallError {
    #[error("send error")]
    Send,

    #[error("failed to format call argument into a message payload")]
    PayloadFormat(#[from] format::Error),
}

pub(crate) type OrderPostError = OrderCallError;

impl From<mpsc::error::SendError<Order>> for OrderCallError {
    fn from(err: mpsc::error::SendError<Order>) -> Self {
        let mpsc::error::SendError(_order) = err;
        Self::Send
    }
}

#[derive(Debug, thiserror::Error)]
#[error("send error")]
pub(crate) struct OrderSendError;

impl From<mpsc::error::SendError<Order>> for OrderSendError {
    fn from(err: mpsc::error::SendError<Order>) -> Self {
        let mpsc::error::SendError(_order) = err;
        Self
    }
}

#[derive(Debug)]
enum Order {
    CallRequest(call::Request),
    CallCancel(call::Cancel),
    Post(Post),
    Event(Event),
}

#[derive(Clone, Debug)]
struct IdGenerator {
    previous: Arc<AtomicU32>,
}

impl IdGenerator {
    fn generate(&self) -> Id {
        let val = self
            .previous
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Id::new(val)
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self {
            previous: Arc::new(AtomicU32::new(1)),
        }
    }
}

pub(crate) mod call {
    use crate::message::{Id, Recipient};
    use tokio::sync::oneshot;
    use tracing::trace;

    pub(super) struct Request {
        pub(super) id: Id,
        pub(super) recipient: Recipient,
        pub(super) payload: Vec<u8>,
        pub(super) result: RequestResultSender,
    }

    impl std::fmt::Debug for Request {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Request")
                .field("id", &self.id)
                .field("recipient", &self.recipient)
                .finish_non_exhaustive()
        }
    }

    #[derive(Debug)]
    pub(super) struct Cancel {
        pub(super) id: Id,
        pub(super) recipient: Recipient,
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

    pub(super) fn result_channel() -> (RequestResultSender, RequestResultReceiver) {
        oneshot::channel()
    }

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub(crate) enum Error {}

    pub(crate) type RequestResult = Result<Response, Error>;
    pub(super) type RequestResultSender = oneshot::Sender<RequestResult>;
    pub(crate) type RequestResultReceiver = oneshot::Receiver<RequestResult>;

    pub(crate) type RequestResultRecvError = oneshot::error::RecvError;
}

#[derive(Debug)]
struct Post {
    recipient: Recipient,
    payload: Vec<u8>,
}

#[derive(Debug)]
struct Event {
    recipient: Recipient,
    payload: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Action, Message, Object, Recipient, Service};
    use assert_matches::assert_matches;
    use futures::{poll, FutureExt, SinkExt, StreamExt};
    use tokio::sync::oneshot;

    #[test]
    fn test_message_id_generator() {
        let gen = IdGenerator::default();
        assert_eq!(gen.generate(), Id::new(1));
        assert_eq!(gen.generate(), Id::new(2));
        assert_eq!(gen.generate(), Id::new(3));
    }

    const RECIPIENT: Recipient = Recipient {
        service: Service::new(1),
        object: Object::new(2),
        action: Action::new(3),
    };

    async fn send_call(
        dispatch: &mut Dispatch,
        client: &OrderSender,
    ) -> (Id, call::RequestResultReceiver) {
        let (id, mut resp_receiver) = client.call_request(RECIPIENT, "hello").await.unwrap();

        // The message is available.
        assert_eq!(
            dispatch.next().await.unwrap(),
            Message {
                id: Id::from(1),
                ty: Type::Call,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o'].into(),
            }
        );

        // The response is still awaited.
        assert_eq!(
            resp_receiver.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        );

        (id, resp_receiver)
    }

    #[tokio::test]
    async fn test_dispatch_drop_dispatch_causes_stream_end() {
        let (mut dispatch, client) = Dispatch::new();
        drop(client);
        assert_eq!(dispatch.next().await, None);
    }

    #[tokio::test]
    async fn test_dispatch_receive_garbage_discarded() {
        let (mut dispatch, _client) = Dispatch::new();

        // Unwanted messages are discarded by the dispatch.
        dispatch
            .send(Message {
                id: Id::from(321),
                ty: Type::Call,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![1, 2, 3].into(),
            })
            .await
            .unwrap();
        dispatch
            .send(Message {
                id: Id::from(3829),
                ty: Type::Reply,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![1, 2, 3].into(),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_dispatch_send_call_receive_response_ok() {
        let (mut dispatch, client) = Dispatch::new();

        let (id, result) = send_call(&mut dispatch, &client).await;

        dispatch
            .send(Message {
                id,
                ty: Type::Reply,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![4, 5, 6].into(),
            })
            .await
            .unwrap();
        assert_eq!(
            result.await.unwrap(),
            Ok(call::Response::Reply(vec![4, 5, 6]))
        );
    }

    #[tokio::test]
    async fn test_dispatch_send_call_receive_response_error() {
        let (mut dispatch, client) = Dispatch::new();

        let (id, result) = send_call(&mut dispatch, &client).await;

        dispatch
            .send(Message {
                id,
                ty: Type::Error,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![4, 5, 6].into(),
            })
            .await
            .unwrap();
        assert_eq!(
            result.await.unwrap(),
            Ok(call::Response::Error(vec![4, 5, 6]))
        );
    }

    #[tokio::test]
    async fn test_dispatch_send_call_receive_response_canceled() {
        let (mut dispatch, client) = Dispatch::new();

        let (id, result) = send_call(&mut dispatch, &client).await;

        dispatch
            .send(Message {
                id,
                ty: Type::Canceled,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![4, 5, 6].into(),
            })
            .await
            .unwrap();
        assert_eq!(result.await.unwrap(), Ok(call::Response::Canceled));
    }

    #[tokio::test]
    async fn test_dispatch_send_call_receive_response_bad_recipient() {
        let (mut dispatch, client) = Dispatch::new();

        let (id, result) = send_call(&mut dispatch, &client).await;

        let recipient = Recipient {
            service: Service::from(100),
            object: Object::from(99),
            action: Action::from(98),
        };
        assert_ne!(recipient, RECIPIENT);

        dispatch
            .send(Message {
                id,
                ty: Type::Reply,
                flags: Flags::empty(),
                recipient,
                payload: vec![4, 5, 6].into(),
            })
            .await
            .unwrap();

        assert_matches!(poll!(result.boxed()), Poll::Pending);
    }

    #[tokio::test]
    async fn test_dispatch_send_call_drop_result_response_discarded() {
        let (mut dispatch, client) = Dispatch::new();
        let (id, result) = send_call(&mut dispatch, &client).await;
        drop(result);
        dispatch
            .send(Message {
                id,
                ty: Type::Reply,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![4, 5, 6].into(),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_dispatch_send_call_drop_dispatch_causes_error() {
        let (mut dispatch, client) = Dispatch::new();
        let (_id, result) = send_call(&mut dispatch, &client).await;
        drop(dispatch);
        assert!(result.await.is_err());
    }

    #[tokio::test]
    async fn test_dispatch_call_cancel() {
        let (mut dispatch, client) = Dispatch::new();

        client.call_cancel(Id::from(32), RECIPIENT).await.unwrap();

        let msg = dispatch.next().await.unwrap();
        assert_eq!(
            msg,
            Message {
                id: Id::from(1),
                ty: Type::Cancel,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![32, 0, 0, 0].into(), // serialized ID of the original message
            }
        );
    }

    #[tokio::test]
    async fn test_dispatch_post() {
        let (mut dispatch, client) = Dispatch::new();

        client.post(RECIPIENT, "hello").await.unwrap();

        let msg = dispatch.next().await.unwrap();
        assert_eq!(
            msg,
            Message {
                id: Id::from(1),
                ty: Type::Post,
                flags: Flags::empty(),
                recipient: RECIPIENT,
                payload: vec![5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o'].into(),
            }
        );
    }
}
