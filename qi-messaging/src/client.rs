use crate::{
    id_factory::SharedIdFactory,
    message::{Address, Id, OnewayRequest, Response},
    Error, Message,
};
use futures::{
    stream::{FusedStream, FuturesUnordered},
    FutureExt, Sink, Stream, StreamExt,
};
use pin_project_lite::pin_project;
use std::{
    collections::HashMap,
    pin::Pin,
    task::{ready, Context, Poll},
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::{CancellationToken, DropGuard, PollSender, WaitForCancellationFutureOwned};

#[derive(Clone)]
pub struct Client<T, R> {
    requests: PollSender<Request<T, R>>,
}

impl<T, R> Client<T, R>
where
    T: Send,
    R: Send,
{
    fn new(requests: mpsc::Sender<Request<T, R>>) -> Self {
        Self {
            requests: PollSender::new(requests),
        }
    }

    pub fn downgrade(&self) -> WeakClient<T, R> {
        WeakClient {
            requests: self.requests.get_ref().map(mpsc::Sender::downgrade),
        }
    }
}

impl<T, R> std::fmt::Debug for Client<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Client")
    }
}

impl<T, R> tower_service::Service<(Address, T)> for Client<T, R>
where
    T: Send,
    R: Send,
{
    type Response = R;
    type Error = Error;
    type Future = CallFuture<T, R>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_reserve(cx).map_err(Into::into)
    }

    fn call(&mut self, (address, value): (Address, T)) -> Self::Future {
        // The resulting PollSender of a `clone` has an initial state identical to calling
        // `PollSender::new`. So to keep the the "send permit" that was allocated with
        // `poll_reserve`, we replace this object Sender with the new one and use the old Sender.
        let new_requests = self.requests.clone();
        let requests = std::mem::replace(&mut self.requests, new_requests);
        CallFuture::new(requests, address, value)
    }
}

impl<T, R> Sink<(Address, OnewayRequest<T>)> for Client<T, R>
where
    T: Send + 'static,
    R: Send,
{
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.poll_reserve(cx).map_err(Into::into)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        (address, request): (Address, OnewayRequest<T>),
    ) -> Result<(), Self::Error> {
        self.requests
            .send_item(Request::Oneway(address, request))
            .map_err(Into::into)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.requests.close();
        Poll::Ready(Ok(()))
    }
}

#[derive(Clone, Default)]
pub struct WeakClient<T, R> {
    requests: Option<mpsc::WeakSender<Request<T, R>>>,
}

impl<T, R> WeakClient<T, R>
where
    T: Send,
    R: Send,
{
    pub fn upgrade(&self) -> Option<Client<T, R>> {
        self.requests
            .as_ref()
            .and_then(mpsc::WeakSender::upgrade)
            .map(|requests| Client::new(requests))
    }
}

impl<T, R> std::fmt::Debug for WeakClient<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WeakClient")
    }
}

#[derive(Debug)]
enum Request<T, R> {
    Call {
        address: Address,
        value: T,
        cancel_token: CancellationToken,
        response_sender: oneshot::Sender<Result<R, Error>>,
    },
    Oneway(Address, OnewayRequest<T>),
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct CallFuture<T, R> {
    inner: CallFutureInner<T, R>,
}

impl<T, R> CallFuture<T, R> {
    fn new(sender: PollSender<Request<T, R>>, address: Address, value: T) -> Self {
        Self {
            inner: CallFutureInner::Ready {
                sender,
                address,
                value: Some(value),
            },
        }
    }
}

impl<T, R> Unpin for CallFuture<T, R> {}

impl<T, R> std::fmt::Debug for CallFuture<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CallFuture")
    }
}

impl<T, R> std::future::Future for CallFuture<T, R>
where
    T: Send,
    R: Send,
{
    type Output = Result<R, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_result(cx)
    }
}

impl<T, R> futures::future::FusedFuture for CallFuture<T, R>
where
    Self: std::future::Future,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

const _: () = {
    fn require_send<T: Send>() {}
    fn require_sync<T: Sync>() {}
    fn require_unpin<T: Unpin>() {}
    fn require_fut_unpin<T, R>() {
        require_unpin::<CallFuture<T, R>>();
    }
    fn require_fut_send<T: Send, R: Send>() {
        require_send::<CallFuture<T, R>>();
    }
    fn require_fut_sync<T: Send + Sync, R: Send>() {
        require_sync::<CallFuture<T, R>>()
    }
};

enum CallFutureInner<T, R> {
    Ready {
        sender: PollSender<Request<T, R>>,
        address: Address,
        value: Option<T>,
    },
    WaitResponse {
        receiver: oneshot::Receiver<Result<R, Error>>,
        drop_guard: Option<DropGuard>,
    },
    Terminated,
}

impl<T, R> CallFutureInner<T, R> {
    fn is_terminated(&self) -> bool {
        matches!(self, Self::Terminated)
    }
}

impl<T, R> CallFutureInner<T, R>
where
    T: Send,
    R: Send,
{
    fn poll_result(&mut self, cx: &mut Context<'_>) -> Poll<Result<R, Error>> {
        loop {
            match self {
                Self::Ready {
                    sender,
                    address,
                    value,
                } => match value.take() {
                    Some(value) => {
                        let (response_sender, response_receiver) = oneshot::channel();
                        let cancel_token = CancellationToken::new();
                        let call = Request::Call {
                            address: *address,
                            value,
                            cancel_token: cancel_token.clone(),
                            response_sender,
                        };
                        match sender.send_item(call) {
                            Ok(()) => {
                                let drop_guard = cancel_token.drop_guard();
                                *self = Self::WaitResponse {
                                    receiver: response_receiver,
                                    drop_guard: Some(drop_guard),
                                }
                            }
                            Err(err) => {
                                *self = Self::Terminated;
                                return Poll::Ready(Err(err.into()));
                            }
                        }
                    }
                    None => *self = Self::Terminated,
                },
                Self::WaitResponse {
                    receiver,
                    drop_guard,
                } => {
                    let result = ready!(receiver.poll_unpin(cx))
                        .map_err(Into::into)
                        .and_then(std::convert::identity);
                    if let Some(g) = drop_guard.take() {
                        g.disarm();
                    }
                    *self = Self::Terminated;
                    break Poll::Ready(result);
                }
                Self::Terminated => break Poll::Pending,
            }
        }
    }
}

pub(crate) struct Requests<T, R> {
    id: SharedIdFactory,
    receiver: Option<mpsc::Receiver<Request<T, R>>>,
    call_response_senders: HashMap<Id, oneshot::Sender<Result<R, Error>>>,
    call_canceled_futures: FuturesUnordered<CallCanceledFuture>,
}

impl<T, R> Requests<T, R> {
    fn new(id: SharedIdFactory, receiver: mpsc::Receiver<Request<T, R>>) -> Self {
        Self {
            id,
            receiver: Some(receiver),
            call_response_senders: HashMap::new(),
            call_canceled_futures: FuturesUnordered::new(),
        }
    }

    pub(crate) fn dispatch_response(&mut self, id: Id, response: Response<R>) {
        if let Some(sender) = self.call_response_senders.remove(&id) {
            let _res = sender.send(match response {
                Response::Reply(value) => Ok(value),
                Response::Error(error) => Err(Error::other(error)),
                Response::Canceled => Err(Error::Canceled),
            });
        }
    }
}

impl<T, R> Unpin for Requests<T, R> {}

impl<T, R> Stream for Requests<T, R> {
    type Item = Message<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check if any call has been canceled.
        if !self.call_canceled_futures.is_terminated() {
            if let Poll::Ready(Some((address, call_id))) =
                self.call_canceled_futures.poll_next_unpin(cx)
            {
                return Poll::Ready(Some(Message::Cancel {
                    id: self.id.create(),
                    address,
                    call_id,
                }));
            }
        }

        match self.receiver {
            Some(ref mut receiver) => match ready!(receiver.poll_recv(cx)) {
                Some(request) => {
                    let id = self.id.create();
                    let message = match request {
                        Request::Call {
                            address,
                            value,
                            cancel_token,
                            response_sender,
                        } => {
                            self.call_response_senders.insert(id, response_sender);
                            self.call_canceled_futures.push(CallCanceledFuture {
                                call_id: id,
                                address,
                                inner: cancel_token.cancelled_owned(),
                            });
                            Message::Call { id, address, value }
                        }
                        Request::Oneway(address, request) => match request {
                            OnewayRequest::Post(value) => Message::Post { id, address, value },
                            OnewayRequest::Event(value) => Message::Event { id, address, value },
                            OnewayRequest::Capabilities(capabilities) => Message::Capabilities {
                                id,
                                address,
                                capabilities,
                            },
                            OnewayRequest::Cancel(call_id) => Message::Cancel {
                                id,
                                address,
                                call_id,
                            },
                        },
                    };
                    Poll::Ready(Some(message))
                }
                None => {
                    self.receiver = None;
                    Poll::Ready(None)
                }
            },
            None => Poll::Ready(None),
        }
    }
}

impl<T, R> FusedStream for Requests<T, R> {
    fn is_terminated(&self) -> bool {
        self.receiver.is_none() && self.call_canceled_futures.is_terminated()
    }
}

pub(crate) fn pair<T, R>(id: SharedIdFactory, capacity: usize) -> (Client<T, R>, Requests<T, R>)
where
    T: Send,
    R: Send,
{
    let (sender, receiver) = mpsc::channel(capacity);
    (Client::new(sender), Requests::new(id, receiver))
}

pin_project! {
    struct CallCanceledFuture {
        address: Address,
        call_id: Id,
        #[pin]
        inner: WaitForCancellationFutureOwned,
    }
}

impl std::future::Future for CallCanceledFuture {
    type Output = (Address, Id);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(cx).map(|_| (*this.address, *this.call_id))
    }
}
