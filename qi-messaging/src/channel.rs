use crate::{
    message::Message,
    service::{client::Client, Request, Service},
};
use futures::{Sink, TryStream};

#[derive(Debug)]
pub(crate) struct Channel<C, S> {
    client: C,
    server: S,
}

impl<C, S> Channel<C, S> {
    pub(crate) fn over_sink_stream<Si, St, Svc>(
        sink: Si,
        stream: St,
        service: Svc,
    ) -> Channel<Client, S>
    where
        Si: Sink<Message>,
        St: TryStream<Ok = Message>,
        Svc: Service,
    {
    }
}

impl<C, S> tower::Service<Request> for Channel<C, S>
where
    C: Service,
{
    type Response = C::Response;
    type Error = C::Error;
    type Future = C::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.client.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        tower::Service::call(&mut self.client, req)
    }
}
