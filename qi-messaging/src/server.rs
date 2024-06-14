use crate::{
    message::{Address, Id, OnewayRequest},
    Handler, Message,
};
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Stream, StreamExt, TryFuture,
};
use pin_project_lite::pin_project;
use qi_value::Dynamic;
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{ready, Context, Poll, Waker},
};

pin_project! {
    pub(super) struct Server<Handler, Future, T> {
        handler: Handler,
        #[pin]
        call_futures: FuturesUnordered<CallFuture<Future>>,
        ph: PhantomData<T>,
    }
}

impl<H, T> Server<H, H::Future, T>
where
    H: Handler<T>,
{
    pub(super) fn new(handler: H) -> Self {
        Self {
            handler,
            call_futures: FuturesUnordered::new(),
            ph: PhantomData,
        }
    }

    pub(super) fn call(&mut self, id: Id, address: Address, value: T) {
        let call_future = self.handler.call(address, value);
        self.call_futures
            .push(CallFuture::new(id, address, call_future));
    }

    pub(super) fn oneway_request(&mut self, address: Address, request: OnewayRequest<T>) {
        let _res = self.handler.oneway_request(address, request); // TODO: log ?
    }

    pub(crate) fn cancel(self: Pin<&mut Self>, id: &Id) {
        for call_future in self.project().call_futures.iter_pin_mut() {
            if &call_future.id == id {
                call_future.cancel()
            }
        }
    }
}

impl<H, T> Stream for Server<H, H::Future, T>
where
    H: Handler<T>,
    H::Error: std::string::ToString,
{
    type Item = Message<H::Reply>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.call_futures.poll_next_unpin(cx)
    }
}

impl<H, T> FusedStream for Server<H, H::Future, T>
where
    H: Handler<T>,
    H::Error: std::string::ToString,
{
    fn is_terminated(&self) -> bool {
        self.call_futures.is_terminated()
    }
}

pin_project! {
    #[derive(Debug)]
    struct CallFuture<F> {
        id: Id,
        address: Address,
        #[pin]
        state: CallResponseFutureState<F>,
    }
}

impl<F> CallFuture<F> {
    fn new(id: Id, address: Address, inner: F) -> Self {
        Self {
            id,
            address,
            state: CallResponseFutureState::Running { inner, waker: None },
        }
    }

    fn cancel(self: Pin<&mut Self>) {
        let mut state = self.project().state;
        if let CallResponseFutureStateProj::Running { waker, .. } = state.as_mut().project() {
            if let Some(waker) = waker.take() {
                waker.wake();
            }
            state.set(CallResponseFutureState::Canceled);
        }
    }
}

impl<F, T, E> Future for CallFuture<F>
where
    F: Future<Output = Result<T, E>>,
    E: std::string::ToString,
{
    type Output = Message<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        let (id, address) = (*this.id, *this.address);
        use CallResponseFutureStateProj as Proj;
        match this.state.as_mut().project() {
            Proj::Running { inner, waker } => {
                *waker = Some(cx.waker().clone());
                let call_result = ready!(inner.try_poll(cx));
                this.state.set(CallResponseFutureState::Terminated);
                match call_result {
                    Ok(reply) => Poll::Ready(Message::Reply {
                        id,
                        address,
                        value: reply,
                    }),
                    Err(error) => Poll::Ready(Message::Error {
                        id,
                        address,
                        error: Dynamic(error.to_string()),
                    }),
                }
            }
            Proj::Canceled => {
                this.state.set(CallResponseFutureState::Terminated);
                Poll::Ready(Message::Canceled { id, address })
            }
            Proj::Terminated => {
                debug_assert!(false, "polling a terminated future");
                Poll::Pending
            }
        }
    }
}

pin_project! {
    #[project = CallResponseFutureStateProj]
    #[derive(Debug)]
    enum CallResponseFutureState<F> {
        Running {
            #[pin]
            inner: F,
            waker: Option<Waker>,
        },
        Canceled,
        Terminated,
    }
}
