use super::service_map::ServiceMap;
use crate::{messaging::message, BinaryValue, Error};
use futures::{future::BoxFuture, FutureExt, Sink};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default)]
pub(super) struct MessagingHandler(Arc<Mutex<ServiceMap>>);

impl MessagingHandler {
    pub(super) fn new(service_map: Arc<Mutex<ServiceMap>>) -> Self {
        Self(service_map)
    }
}

impl tower::Service<(message::Address, BinaryValue)> for MessagingHandler {
    type Response = BinaryValue;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, (address, value): (message::Address, BinaryValue)) -> Self::Future {
        let service_map = Arc::clone(&self.0);
        async move { service_map.lock_owned().await.call(address, value).await }.boxed()
    }
}

impl Sink<(message::Address, message::OnewayRequest<BinaryValue>)> for MessagingHandler {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        (address, request): (message::Address, message::OnewayRequest<BinaryValue>),
    ) -> Result<(), Self::Error> {
        match request {
            message::OnewayRequest::Post(post) => todo!(),
            message::OnewayRequest::Event(event) => todo!(),
            message::OnewayRequest::Capabilities(capabilities) => todo!(),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
