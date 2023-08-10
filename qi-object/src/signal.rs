use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::StreamExt;

#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    derive_more::From,
    derive_more::Into,
)]
pub struct Link(u64);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Subscription<T> {
    link: Link,
    phantom: PhantomData<T>,
}

impl<T> futures::Stream for Subscription<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionClient<T> {
    link: Link,
    phantom: PhantomData<T>,
}

impl<T> futures::Stream for SubscriptionClient<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}

#[derive(Debug)]
pub enum AnySubscription<T> {
    Local(Subscription<T>),
    Client(SubscriptionClient<T>),
}

impl<T> futures::Stream for AnySubscription<T>
where
    T: Unpin,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            AnySubscription::Local(local) => local.poll_next_unpin(cx),
            AnySubscription::Client(client) => client.poll_next_unpin(cx),
        }
    }
}
