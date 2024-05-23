use crate::{
    message::{Address, Id},
    Message,
};
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Stream, TryFuture,
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
    pub(crate) struct Responses<Handler, Future, T> {
        handler: Handler,
        #[pin]
        call_futures: FuturesUnordered<CallResponseFuture<Future>>,
        ph: PhantomData<T>,
    }
}

impl<Handler, T> Responses<Handler, Handler::Future, T>
where
    Handler: tower_service::Service<(Address, T)>,
{
    pub(crate) fn new(handler: Handler) -> Self {
        Self {
            handler,
            call_futures: FuturesUnordered::new(),
            ph: PhantomData,
        }
    }

    pub(crate) fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Handler::Error>> {
        self.handler.poll_ready(cx)
    }

    pub(crate) fn call(&mut self, id: Id, (address, value): (Address, T)) {
        let future = self.handler.call((address, value));
        self.call_futures
            .push(CallResponseFuture::new(id, address, future));
    }

    pub(crate) fn cancel(self: Pin<&mut Self>, id: Id) {
        for mut call_future in self.project().call_futures.iter_pin_mut() {
            if call_future.as_mut().id == id {
                call_future.cancel()
            }
        }
    }
}

impl<Svc, T> Stream for Responses<Svc, Svc::Future, T>
where
    Svc: tower_service::Service<(Address, T)>,
    Svc::Error: std::string::ToString,
{
    type Item = Message<Svc::Response>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().call_futures.poll_next(cx)
    }
}

impl<Svc, T> FusedStream for Responses<Svc, Svc::Future, T>
where
    Svc: tower_service::Service<(Address, T)>,
    Svc::Error: std::string::ToString,
{
    fn is_terminated(&self) -> bool {
        self.call_futures.is_terminated()
    }
}

pin_project! {
    #[derive(Debug)]
    struct CallResponseFuture<F> {
        id: Id,
        address: Address,
        #[pin]
        state: CallResponseFutureState<F>,
    }
}

impl<F> CallResponseFuture<F> {
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

impl<F, T, E> Future for CallResponseFuture<F>
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
