use crate::{
    handler,
    message::{Address, Id},
    Message,
};
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

pub(super) struct CallFutures<'a, T, E> {
    call_futures: FuturesUnordered<CallFuture<'a, T, E>>,
}

impl<'a, T, E> Default for CallFutures<'a, T, E> {
    fn default() -> Self {
        Self {
            call_futures: Default::default(),
        }
    }
}

impl<'a, T, E> CallFutures<'a, T, E> {
    pub(super) fn push<F>(&mut self, id: Id, address: Address, future: F)
    where
        F: Future<Output = Result<T, E>> + Send + 'a,
    {
        self.call_futures
            .push(CallFuture::new(id, address, future.boxed()));
    }

    pub(crate) fn cancel(&mut self, id: &Id) {
        for call_future in self.call_futures.iter_mut() {
            if &call_future.id == id {
                call_future.cancel()
            }
        }
    }
}

impl<'a, T, E> Stream for CallFutures<'a, T, E>
where
    E: handler::Error,
{
    /// Message x StopDispatch
    type Item = (Message<T>, bool);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.call_futures.poll_next_unpin(cx)
    }
}

impl<'a, T, E> FusedStream for CallFutures<'a, T, E>
where
    E: handler::Error,
{
    fn is_terminated(&self) -> bool {
        self.call_futures.is_terminated()
    }
}

#[derive(Debug)]
struct CallFuture<'a, T, E> {
    id: Id,
    address: Address,
    state: CallResponseFutureState<'a, T, E>,
}

impl<'a, T, E> CallFuture<'a, T, E> {
    fn new(id: Id, address: Address, inner: BoxFuture<'a, Result<T, E>>) -> Self {
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

impl<'a, T, E> Future for CallFuture<'a, T, E>
where
    E: handler::Error,
{
    /// Message x StopDispatch
    type Output = (Message<T>, bool);

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
                            value: reply,
                        },
                        false,
                    )),
                    // TODO: Check if error corresponds to a "cancelled" request, so that we may
                    // make a Canceled message instead.
                    Err(error) => {
                        let message_stop_pair = if error.is_canceled() {
                            (
                                Message::Canceled {
                                    id: self.id,
                                    address: self.address,
                                },
                                false,
                            )
                        } else {
                            (
                                Message::Error {
                                    id: self.id,
                                    address: self.address,
                                    error: error.to_string(),
                                },
                                error.is_fatal(),
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
                    false,
                ))
            }
            CallResponseFutureState::Terminated => {
                debug_assert!(false, "polling a terminated future");
                Poll::Pending
            }
        }
    }
}

enum CallResponseFutureState<'a, T, E> {
    Running {
        inner: BoxFuture<'a, Result<T, E>>,
        waker: Option<Waker>,
    },
    Canceled,
    Terminated,
}

impl<'a, T, E> std::fmt::Debug for CallResponseFutureState<'a, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running { waker, .. } => f.debug_struct("Running").field("waker", waker).finish(),
            Self::Canceled => write!(f, "Canceled"),
            Self::Terminated => write!(f, "Terminated"),
        }
    }
}
