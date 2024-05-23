use crate::{
    client,
    message::{Address, Message, OnewayRequest, Response},
    server, Error,
};
use futures::{stream::FusedStream, Sink, SinkExt, Stream, TryStream};
use pin_project_lite::pin_project;
use qi_value::Dynamic;
use std::{
    marker::PhantomData,
    mem::replace,
    pin::Pin,
    task::{ready, Context, Poll},
};

pin_project! {
    // A stream of outgoing messages of an endpoint.
    //
    // It selects between two stream sides:
    //   - incoming messages,
    //   - client requests,
    //
    // Client requests are the source of outgoing messages of types Call, Post, Event, Capabilities and
    // Cancel. They originate only from clients objects request channel but never from the incoming
    // messages.
    //
    // Server responses are the source of outgoing messages of types Reply, Error and
    // Canceled. They originate from a sequencing between incoming messages and handler calls.
    //
    // Incoming messages also have side-effects on the results sent to clients.
    //
    //                ┌───────────────────────┐
    //                │                       │
    //                │   Incoming Messages   │
    //                │                       │
    //                └───────────┬───────────┘
    //                            │
    //    ┌─────┬────┬─────┬──────┴──┬────────┬──────┬───────┐
    //    │     │    │     │         │        │      │       │
    //  Call Cancel Post Event Capabilities Reply Canceled Error
    //    │     │    │     │         │        │      │       │              ┌─────────┐
    // ┌──▼─────▼────▼─────▼─────────▼─────┬──▼──────▼───────▼──┐           │         ├┐
    // │                                   │                    │           │ Clients ││
    // │              Request              │      Response      │           │         ││
    // │                                   │                    │           └┬────────┘│
    // └──┬─────┬────┬─────┬─────────┬─────┴──┬──────┬───────┬──┘            └─────────┘
    //    │     │    │     │         │        │      │       │     Call Cancel Post Event Capababilities
    //  ┌─▼─────▼─┬──▼─────▼─────────▼────┐ ┌─▼──────▼───────▼─┐     │    │     │     │         │
    //  │         │                       │ │                  │     │    │     │     │         │
    //  │ Handler │        Oneway         │ │      Client      ◄─────┴────┴─────┴─────┴─────────┘
    //  │  Calls  │         Sink          │ │     Requests     │
    //  │         │                       │ │                  │
    //  └────┬────┴───────────────────────┘ └────────┬─────────┘
    //    Server                                     │
    //   Responses                                   │
    //       └───────────────┐      ┌────────────────┘
    //                       │      │
    //               ┌───────▼──────▼────────┐
    //               │                       │
    //               │   Outgoing Messages   │
    //               │                       │
    //               └───────────────────────┘
    pub(crate) struct OutgoingMessages<Msg, Handler, SvcFuture, Snk, InBody, OutBody> {
        poll_side: PollState,

        #[pin]
        inner: Inner<Msg, Handler, SvcFuture, Snk, InBody, OutBody>,
    }
}

impl<Msgs, Handler, Snk, InBody, OutBody>
    OutgoingMessages<Msgs, Handler, Handler::Future, Snk, InBody, OutBody>
where
    Msgs: TryStream<Ok = Message<InBody>>,
    Handler: tower_service::Service<(Address, InBody)>,
{
    pub(super) fn new(
        incoming_messages: Msgs,
        server_responses: server::Responses<Handler, Handler::Future, InBody>,
        server_oneway_sink: Snk,
        client_requests: client::Requests<OutBody, InBody>,
    ) -> Self {
        Self {
            poll_side: PollState::ServerResponses,
            inner: Inner {
                incoming_messages,
                dispatch_state: DispatchState::NotReady,
                server_responses,
                server_oneway_sink,
                client_requests,
                phantom: PhantomData,
            },
        }
    }
}

impl<Msgs, Handler, Snk, InBody, OutBody> Stream
    for OutgoingMessages<Msgs, Handler, Handler::Future, Snk, InBody, OutBody>
where
    Msgs: TryStream<Ok = Message<InBody>>,
    Msgs::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    Handler: tower_service::Service<(Address, InBody), Response = OutBody>,
    Handler::Error: std::string::ToString + Into<Box<dyn std::error::Error + Send + Sync>>,
    Snk: Sink<(Address, OnewayRequest<InBody>)>,
    Snk::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Item = Result<Message<OutBody>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        // Dispatch incoming messages as much as possible. Ignore the pending result as it is not a
        // source of outgoing messages anyway.
        let dispatch_is_terminated = this.inner.as_mut().poll_dispatch(cx)?.is_ready();

        // Poll branches until one produces a message, both are pending or both are terminated.
        let mut server_responses_terminated = None;
        let mut client_requests_terminated = None;
        loop {
            if let Some((server_responses_terminated, client_requests_terminated)) =
                server_responses_terminated.zip(client_requests_terminated)
            {
                // Server responses can become temporarily terminated, and as long as the
                // dispatch is not terminated, it can become active again.
                // This means that in order to be fully terminated, the dispatch needs to be
                // terminated as well.
                break if dispatch_is_terminated
                    && server_responses_terminated
                    && client_requests_terminated
                {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                };
            }
            match this.poll_side.switch_next() {
                PollState::ServerResponses => match this.inner.as_mut().poll_server_responses(cx) {
                    Poll::Pending => server_responses_terminated = Some(false),
                    Poll::Ready(None) => {
                        debug_assert!(this.inner.server_responses.is_terminated());
                        server_responses_terminated = Some(true);
                    }
                    Poll::Ready(Some(message)) => return Poll::Ready(Some(Ok(message))),
                },
                PollState::ClientRequests => match this.inner.as_mut().poll_client_requests(cx) {
                    Poll::Pending => client_requests_terminated = Some(false),
                    Poll::Ready(None) => {
                        debug_assert!(this.inner.client_requests.is_terminated());
                        client_requests_terminated = Some(true);
                    }
                    Poll::Ready(Some(message)) => return Poll::Ready(Some(Ok(message))),
                },
            }
        }
    }
}

impl<Msgs, Svc, Sink, InBody, OutBody> FusedStream
    for OutgoingMessages<Msgs, Svc, Svc::Future, Sink, InBody, OutBody>
where
    Self: Stream,
    Svc: tower_service::Service<(Address, InBody)>,
    server::Responses<Svc, Svc::Future, InBody>: FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.inner.dispatch_state == DispatchState::Terminated
            && self.inner.server_responses.is_terminated()
            && self.inner.client_requests.is_terminated()
    }
}
pin_project! {
    struct Inner<Msgs, Svc, SvcFuture, Sink, InBody, OutBody> {
        #[pin]
        incoming_messages: Msgs,
        dispatch_state: DispatchState,
        #[pin]
        server_responses: server::Responses<Svc, SvcFuture, InBody>,
        #[pin]
        server_oneway_sink: Sink,
        #[pin]
        client_requests: client::Requests<OutBody, InBody>,
        phantom: PhantomData<fn() -> OutBody>,
    }
}

impl<Msgs, Svc, Snk, InBody, OutBody> Inner<Msgs, Svc, Svc::Future, Snk, InBody, OutBody>
where
    Msgs: TryStream<Ok = Message<InBody>>,
    Msgs::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    Svc: tower_service::Service<(Address, InBody), Response = OutBody>,
    Svc::Error: std::string::ToString + Into<Box<dyn std::error::Error + Send + Sync>>,
    Snk: Sink<(Address, OnewayRequest<InBody>)>,
    Snk::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn poll_dispatch(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let mut this = self.as_mut().project();
        let mut server_oneway_sink = this.server_oneway_sink.sink_map_err(Error::other);
        let mut sent_to_sink = false;
        loop {
            match this.dispatch_state {
                DispatchState::NotReady => {
                    ready!(this.server_responses.poll_ready(cx)).map_err(Error::other)?;
                    ready!(server_oneway_sink.poll_ready_unpin(cx))?;
                    *this.dispatch_state = DispatchState::Ready;
                }
                DispatchState::Ready => {
                    match this
                        .incoming_messages
                        .as_mut()
                        .try_poll_next(cx)
                        .map_err(Error::other)?
                    {
                        Poll::Pending => {
                            if sent_to_sink {
                                *this.dispatch_state = DispatchState::PendingFlushSink;
                            } else {
                                // Revert to a state of non-readiness so that we need to poll the
                                // handler and sink for readiness again next time.
                                *this.dispatch_state = DispatchState::NotReady;
                                break Poll::Pending;
                            }
                        }
                        Poll::Ready(None) => {
                            *this.dispatch_state = DispatchState::TerminatedCloseSink
                        }
                        Poll::Ready(Some(message)) => match message {
                            Message::Call { id, address, value } => {
                                this.server_responses.call(id, (address, value));
                                *this.dispatch_state = DispatchState::NotReady;
                            }
                            Message::Cancel { call_id, .. } => {
                                this.server_responses.as_mut().cancel(call_id)
                            }
                            Message::Event { address, value, .. } => {
                                server_oneway_sink
                                    .start_send_unpin((address, OnewayRequest::Event(value)))?;
                                sent_to_sink = true;
                                *this.dispatch_state = DispatchState::NotReady;
                            }
                            Message::Post { address, value, .. } => {
                                server_oneway_sink
                                    .start_send_unpin((address, OnewayRequest::Post(value)))?;
                                sent_to_sink = true;
                                *this.dispatch_state = DispatchState::NotReady;
                            }
                            Message::Capabilities {
                                address,
                                capabilities,
                                ..
                            } => {
                                server_oneway_sink.start_send_unpin((
                                    address,
                                    OnewayRequest::Capabilities(capabilities),
                                ))?;
                                sent_to_sink = true;
                                *this.dispatch_state = DispatchState::NotReady;
                            }
                            Message::Reply { id, value, .. } => this
                                .client_requests
                                .dispatch_response(id, Response::Reply(value)),
                            Message::Error {
                                id,
                                error: Dynamic(error),
                                ..
                            } => this
                                .client_requests
                                .dispatch_response(id, Response::Error(error)),
                            Message::Canceled { id, .. } => this
                                .client_requests
                                .dispatch_response(id, Response::Canceled),
                        },
                    }
                }
                DispatchState::PendingFlushSink => {
                    ready!(server_oneway_sink
                        .poll_flush_unpin(cx)
                        .map_err(Error::other)?);
                    *this.dispatch_state = DispatchState::NotReady;
                    break Poll::Pending;
                }
                DispatchState::TerminatedCloseSink => {
                    ready!(server_oneway_sink
                        .poll_close_unpin(cx)
                        .map_err(Error::other)?);
                    *this.dispatch_state = DispatchState::Terminated;
                }
                DispatchState::Terminated => break Poll::Ready(Ok(())),
            }
        }
    }

    fn poll_server_responses(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Message<OutBody>>> {
        let this = self.project();
        if this.server_responses.is_terminated() {
            Poll::Ready(None)
        } else {
            this.server_responses.poll_next(cx)
        }
    }

    fn poll_client_requests(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Message<OutBody>>> {
        let this = self.project();
        if this.client_requests.is_terminated() {
            Poll::Ready(None)
        } else {
            this.client_requests.poll_next(cx)
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum PollState {
    #[default]
    ServerResponses,
    ClientRequests,
}

impl PollState {
    fn switch_next(&mut self) -> PollState {
        match self {
            Self::ServerResponses => replace(self, Self::ClientRequests),
            Self::ClientRequests => replace(self, Self::ServerResponses),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum DispatchState {
    #[default]
    NotReady,
    Ready,
    PendingFlushSink,
    TerminatedCloseSink,
    Terminated,
}
