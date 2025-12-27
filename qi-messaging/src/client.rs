use crate::{
    id::CreateId,
    message::{Address, Id, Response},
    Error, Message,
};
use bytes::Bytes;
use futures::{stream::FusedStream, Stream};
use std::{
    collections::HashMap,
    pin::Pin,
    task::{ready, Context, Poll},
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct Client {
    requests: mpsc::Sender<Request>,
}

impl Client {
    fn new(requests: mpsc::Sender<Request>) -> Self {
        Self { requests }
    }

    pub fn downgrade(&self) -> WeakClient {
        WeakClient {
            requests: self.requests.downgrade(),
        }
    }

    pub async fn call(&self, address: Address, args: Bytes) -> Result<Bytes, Error> {
        let request_permit = self
            .requests
            .reserve()
            .await
            .map_err(client_dissociated_with_endpoint_error)?;
        let (response_sender, response_receiver) = oneshot::channel();
        let cancel_token = CancellationToken::new();
        let drop_guard = cancel_token.clone().drop_guard();
        request_permit.send(Request::Call {
            address,
            args,
            cancel_token,
            response_sender,
        });
        let response = response_receiver
            .await
            .map_err(client_dissociated_with_endpoint_error);
        drop_guard.disarm();
        response?
    }

    pub async fn send_event(&self, address: Address, args: Bytes) -> Result<(), Error> {
        self.requests
            .send(Request::Event { address, args })
            .await
            .map_err(client_dissociated_with_endpoint_error)
    }

    pub async fn post(&self, address: Address, args: Bytes) -> Result<(), Error> {
        self.requests
            .send(Request::Post { address, args })
            .await
            .map_err(client_dissociated_with_endpoint_error)
    }
}

#[derive(Debug, Clone)]
pub struct WeakClient {
    requests: mpsc::WeakSender<Request>,
}

impl WeakClient {
    pub fn upgrade(&self) -> Option<Client> {
        self.requests.upgrade().map(Client::new)
    }
}

fn client_dissociated_with_endpoint_error<E>(_err: E) -> Error {
    Error::LinkLost("the client has been dissociated with the messaging loop".into())
}

#[derive(Debug)]
enum Request {
    Call {
        address: Address,
        args: Bytes,
        cancel_token: CancellationToken,
        response_sender: oneshot::Sender<Result<Bytes, Error>>,
    },
    Post {
        address: Address,
        args: Bytes,
    },
    Event {
        address: Address,
        args: Bytes,
    },
}

/// Creates a client and a stream of its requests.
pub(crate) fn new_with_requests(requests_buffer_capacity: usize) -> (Client, Requests) {
    let (sender, receiver) = mpsc::channel(requests_buffer_capacity);
    (Client::new(sender), Requests::new(receiver))
}

#[derive(Debug)]
pub(crate) struct Requests {
    id: CreateId,
    receiver: Option<mpsc::Receiver<Request>>,
    running_calls: HashMap<Id, CallState>,
}

impl Requests {
    fn new(receiver: mpsc::Receiver<Request>) -> Self {
        Self {
            id: CreateId::default(),
            receiver: Some(receiver),
            running_calls: HashMap::new(),
        }
    }

    pub(super) fn dispatch_response(&mut self, id: Id, response: Response) {
        if let Some(CallState {
            response_sender, ..
        }) = self.running_calls.remove(&id)
        {
            let _res = response_sender.send(match response {
                Response::Reply(value) => Ok(value),
                Response::Error(error) => Err(Error::CallError(error)),
                Response::Canceled => Err(Error::CallCanceled),
            });
        }
    }
}

impl Stream for Requests {
    type Item = Message;

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
                            args,
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
                            Message::Call {
                                id,
                                address,
                                payload: args,
                            }
                        }
                        Request::Post { address, args } => Message::Post {
                            id,
                            address,
                            payload: args,
                        },
                        Request::Event { address, args } => Message::Event {
                            id,
                            address,
                            payload: args,
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

impl FusedStream for Requests {
    fn is_terminated(&self) -> bool {
        self.receiver.is_none()
    }
}

#[derive(Debug)]
struct CallState {
    id: Id,
    address: Address,
    response_sender: oneshot::Sender<Result<Bytes, Error>>,
    cancel_token: CancellationToken,
}
