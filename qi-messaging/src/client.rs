use crate::{
    id_factory::SharedIdFactory,
    message::{Address, FireAndForget, Id, Response},
    value::KeyDynValueMap,
    Error, Message,
};
use futures::{stream::FusedStream, Stream};
use std::{
    collections::HashMap,
    pin::Pin,
    task::{ready, Context, Poll},
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub struct Client<Body> {
    requests: mpsc::Sender<Request<Body>>,
}

impl<Body> Client<Body> {
    fn new(requests: mpsc::Sender<Request<Body>>) -> Self {
        Self { requests }
    }

    pub fn downgrade(&self) -> WeakClient<Body> {
        WeakClient {
            requests: self.requests.downgrade(),
        }
    }

    pub async fn call(&self, address: Address, value: Body) -> Result<Body, Error> {
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
            value,
            cancel_token,
            response_sender,
        });
        let response = response_receiver
            .await
            .map_err(client_dissociated_with_endpoint_error);
        drop_guard.disarm();
        response?
    }

    pub async fn fire_and_forget(
        &self,
        address: Address,
        request: FireAndForget<Body>,
    ) -> Result<(), Error> {
        let request = match request {
            FireAndForget::Capabilities(capabilities) => Request::Capababilities {
                address,
                capabilities,
            },
            FireAndForget::Post(value) => Request::Post { address, value },
            FireAndForget::Event(value) => Request::Event { address, value },
        };
        self.requests
            .send(request)
            .await
            .map_err(client_dissociated_with_endpoint_error)
    }
}

fn client_dissociated_with_endpoint_error<E>(_err: E) -> Error {
    Error::LinkLost("the client has been dissociated with the messaging loop".into())
}

impl<Body> Clone for Client<Body> {
    fn clone(&self) -> Self {
        Self {
            requests: self.requests.clone(),
        }
    }
}

impl<Body> std::fmt::Debug for Client<Body> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("requests", &self.requests)
            .finish()
    }
}

pub struct WeakClient<Body> {
    requests: mpsc::WeakSender<Request<Body>>,
}

impl<Body> WeakClient<Body> {
    pub fn upgrade(&self) -> Option<Client<Body>> {
        self.requests.upgrade().map(Client::new)
    }
}

impl<Body> Clone for WeakClient<Body> {
    fn clone(&self) -> Self {
        Self {
            requests: self.requests.clone(),
        }
    }
}

impl<Body> std::fmt::Debug for WeakClient<Body> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakClient")
            .field("requests", &self.requests)
            .finish()
    }
}

#[derive(Debug)]
enum Request<Body> {
    Call {
        address: Address,
        value: Body,
        cancel_token: CancellationToken,
        response_sender: oneshot::Sender<Result<Body, Error>>,
    },
    Post {
        address: Address,
        value: Body,
    },
    Event {
        address: Address,
        value: Body,
    },
    Capababilities {
        address: Address,
        capabilities: KeyDynValueMap,
    },
}

pub(crate) struct Requests<Body> {
    id: SharedIdFactory,
    receiver: Option<mpsc::Receiver<Request<Body>>>,
    running_calls: HashMap<Id, CallState<Body>>,
}

impl<Body> Requests<Body> {
    fn new(id: SharedIdFactory, receiver: mpsc::Receiver<Request<Body>>) -> Self {
        Self {
            id,
            receiver: Some(receiver),
            running_calls: HashMap::new(),
        }
    }

    pub(super) fn dispatch_response(&mut self, id: Id, response: Response<Body>) {
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

impl<Body> Unpin for Requests<Body> {}

impl<Body> Stream for Requests<Body> {
    type Item = Message<Body>;

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

impl<Body> FusedStream for Requests<Body> {
    fn is_terminated(&self) -> bool {
        self.receiver.is_none()
    }
}

pub(crate) fn pair<Body>(id: SharedIdFactory, capacity: usize) -> (Client<Body>, Requests<Body>) {
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
