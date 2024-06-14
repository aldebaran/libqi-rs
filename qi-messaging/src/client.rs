use crate::{
    id_factory::SharedIdFactory,
    message::{Address, Id, OnewayRequest, Response},
    CapabilitiesMap, Error, Message,
};
use futures::{stream::FusedStream, Stream};
use std::{
    collections::HashMap,
    pin::Pin,
    task::{ready, Context, Poll},
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct Client<T, R> {
    requests: mpsc::Sender<Request<T, R>>,
}

impl<T, R> Client<T, R>
where
    T: Send,
    R: Send,
{
    fn new(requests: mpsc::Sender<Request<T, R>>) -> Self {
        Self { requests }
    }

    pub fn downgrade(&self) -> WeakClient<T, R> {
        WeakClient {
            requests: self.requests.downgrade(),
        }
    }

    pub async fn call(&self, address: Address, value: T) -> Result<R, Error> {
        let request_permit = self.requests.reserve().await?;
        let (response_sender, response_receiver) = oneshot::channel();
        let cancel_token = CancellationToken::new();
        let drop_guard = cancel_token.clone().drop_guard();
        request_permit.send(Request::Call {
            address,
            value,
            cancel_token,
            response_sender,
        });
        let response = response_receiver.await;
        drop_guard.disarm();
        response?
    }

    pub async fn oneway(&self, address: Address, request: OnewayRequest<T>) -> Result<(), Error> {
        let request = match request {
            OnewayRequest::Capabilities(capabilities) => Request::Capababilities {
                address,
                capabilities,
            },
            OnewayRequest::Post(value) => Request::Post { address, value },
            OnewayRequest::Event(value) => Request::Event { address, value },
        };
        self.requests.send(request).await.map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct WeakClient<T, R> {
    requests: mpsc::WeakSender<Request<T, R>>,
}

impl<T, R> WeakClient<T, R>
where
    T: Send,
    R: Send,
{
    pub fn upgrade(&self) -> Option<Client<T, R>> {
        self.requests.upgrade().map(Client::new)
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
    Post {
        address: Address,
        value: T,
    },
    Event {
        address: Address,
        value: T,
    },
    Capababilities {
        address: Address,
        capabilities: CapabilitiesMap<'static>,
    },
}

pub(crate) struct Requests<T, R> {
    id: SharedIdFactory,
    receiver: Option<mpsc::Receiver<Request<T, R>>>,
    running_calls: HashMap<Id, CallState<R>>,
}

impl<T, R> Requests<T, R> {
    fn new(id: SharedIdFactory, receiver: mpsc::Receiver<Request<T, R>>) -> Self {
        Self {
            id,
            receiver: Some(receiver),
            running_calls: HashMap::new(),
        }
    }

    pub(super) fn dispatch_response(&mut self, id: Id, response: Response<R>) {
        if let Some(CallState {
            response_sender, ..
        }) = self.running_calls.remove(&id)
        {
            let _res = response_sender.send(match response {
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
        if let Some(id) = self
            .running_calls
            .iter()
            .find_map(|(id, call)| call.cancel_token.is_cancelled().then_some(id))
            .copied()
        {
            let CallState {
                id: call_id,
                address,
                ..
            } = self.running_calls.remove(&id).unwrap();
            return Poll::Ready(Some(Message::Cancel {
                id: self.id.create(),
                address,
                call_id,
            }));
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
                            self.running_calls.insert(
                                id,
                                CallState {
                                    id,
                                    address,
                                    response_sender,
                                    cancel_token,
                                },
                            );
                            Message::Call { id, address, value }
                        }
                        Request::Post { address, value } => Message::Post { id, address, value },
                        Request::Event { address, value } => Message::Event { id, address, value },
                        Request::Capababilities {
                            address,
                            capabilities,
                        } => Message::Capabilities {
                            id,
                            address,
                            capabilities,
                        },
                    };
                    Poll::Ready(Some(message))
                }
                None => {
                    self.receiver = None;
                    self.running_calls.clear();
                    Poll::Ready(None)
                }
            },
            None => Poll::Ready(None),
        }
    }
}

impl<T, R> FusedStream for Requests<T, R> {
    fn is_terminated(&self) -> bool {
        self.receiver.is_none()
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

#[derive(Debug)]
struct CallState<T> {
    id: Id,
    address: Address,
    response_sender: oneshot::Sender<Result<T, Error>>,
    cancel_token: CancellationToken,
}
