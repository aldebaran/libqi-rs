use crate::{
    handler,
    message::{Address, Id},
    Message,
};
use bytes::Bytes;
use futures::{
    future::BoxFuture,
    stream::{FusedStream, FuturesUnordered},
    FutureExt, Stream, StreamExt, TryFutureExt,
};
use std::{
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll, Waker},
};

#[derive(Debug)]
pub(super) struct CallFutures<E> {
    call_futures: FuturesUnordered<CallFuture<E>>,
}

impl<E> Default for CallFutures<E> {
    fn default() -> Self {
        Self {
            call_futures: Default::default(),
        }
    }
}

impl<E> CallFutures<E> {
    pub(super) fn push<F>(&mut self, id: Id, address: Address, future: F)
    where
        F: Future<Output = Result<Bytes, E>> + Send + 'static,
    {
        self.call_futures
            .push(CallFuture::new(id, address, future.boxed()));
    }

    pub(super) fn cancel(&mut self, id: &Id) {
        for call_future in self.call_futures.iter_mut() {
            if &call_future.id == id {
                call_future.cancel()
            }
        }
    }
}

impl<E> Stream for CallFutures<E>
where
    E: handler::CallError,
{
    type Item = (Message, DispatchFlow);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.call_futures.poll_next_unpin(cx)
    }
}

impl<E> FusedStream for CallFutures<E>
where
    E: handler::CallError,
{
    fn is_terminated(&self) -> bool {
        self.call_futures.is_terminated()
    }
}

#[derive(Debug)]
struct CallFuture<E> {
    id: Id,
    address: Address,
    state: CallResponseFutureState<E>,
}

impl<E> CallFuture<E> {
    fn new(id: Id, address: Address, inner: BoxFuture<'static, Result<Bytes, E>>) -> Self {
        Self {
            id,
            address,
            state: CallResponseFutureState::Running { inner, waker: None },
        }
    }

    fn cancel(&mut self) {
        if let CallResponseFutureState::Running { ref mut waker, .. } = self.state {
            if let Some(waker) = waker.take() {
                waker.wake();
            }
            self.state = CallResponseFutureState::Canceled;
        }
    }
}

impl<E> Future for CallFuture<E>
where
    E: handler::CallError,
{
    type Output = (Message, DispatchFlow);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            CallResponseFutureState::Running {
                ref mut inner,
                ref mut waker,
            } => {
                *waker = Some(cx.waker().clone());
                let call_result = ready!(inner.try_poll_unpin(cx));
                self.state = CallResponseFutureState::Terminated;
                match call_result {
                    Ok(reply) => Poll::Ready((
                        Message::Reply {
                            id: self.id,
                            address: self.address,
                            payload: reply,
                        },
                        DispatchFlow::Continue,
                    )),
                    Err(error) => {
                        let message_stop_pair = if error.is_canceled() {
                            (
                                Message::Canceled {
                                    id: self.id,
                                    address: self.address,
                                },
                                DispatchFlow::Continue,
                            )
                        } else {
                            (
                                Message::Error {
                                    id: self.id,
                                    address: self.address,
                                    error: error.to_string(),
                                },
                                if error.is_fatal() {
                                    DispatchFlow::Stop
                                } else {
                                    DispatchFlow::Continue
                                },
                            )
                        };
                        Poll::Ready(message_stop_pair)
                    }
                }
            }
            CallResponseFutureState::Canceled => {
                self.state = CallResponseFutureState::Terminated;
                Poll::Ready((
                    Message::Canceled {
                        id: self.id,
                        address: self.address,
                    },
                    DispatchFlow::Continue,
                ))
            }
            CallResponseFutureState::Terminated => {
                debug_assert!(false, "polling a terminated future");
                Poll::Pending
            }
        }
    }
}

enum CallResponseFutureState<E> {
    Running {
        inner: BoxFuture<'static, Result<Bytes, E>>,
        waker: Option<Waker>,
    },
    Canceled,
    Terminated,
}

impl<E> std::fmt::Debug for CallResponseFutureState<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running { waker, .. } => f.debug_struct("Running").field("waker", waker).finish(),
            Self::Canceled => write!(f, "Canceled"),
            Self::Terminated => write!(f, "Terminated"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum DispatchFlow {
    Continue,
    Stop,
}
