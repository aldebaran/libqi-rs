use crate::messaging::{
    self, Call, CallTermination, CallWithId, Notification, NotificationWithId, Reply, RequestId,
    RequestWithId, Service, ToRequestId, WithRequestId,
};
use futures::{ready, FutureExt, Sink, SinkExt, Stream, StreamExt};
use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    task::{Context, Poll},
};
use tokio::{
    pin, select,
    sync::{mpsc, oneshot},
};
use tokio_util::sync::PollSender;
use tracing::debug;

pub(crate) fn setup<Si, St>(
    responses_stream: St,
    requests_sink: Si,
) -> (Client, impl Future<Output = Result<(), Si::Error>>)
where
    Si: Sink<RequestWithId>,
    Si::Error: std::error::Error,
    St: Stream<Item = (RequestId, Result<Reply, CallTermination<messaging::Error>>)>,
{
    const DISPATCH_CHANNEL_SIZE: usize = 1;
    let (dispatch_sender, dispatch_receiver) = mpsc::channel(DISPATCH_CHANNEL_SIZE);
    let dispatch_sender = PollSender::new(dispatch_sender);
    let dispatch = dispatch(dispatch_receiver, requests_sink, responses_stream);
    (
        Client {
            dispatch_request_sender: dispatch_sender,
        },
        dispatch,
    )
}

#[derive(Debug, Clone)]
pub(crate) struct Client {
    dispatch_request_sender: PollSender<DispatchRequest>,
}

impl Client {
    fn take_sender(&mut self) -> PollSender<DispatchRequest> {
        let clone = self.dispatch_request_sender.clone();
        std::mem::replace(&mut self.dispatch_request_sender, clone)
    }
}

impl Service<CallWithId, NotificationWithId> for Client {
    type Error = Error;
    type CallFuture = CallFuture;
    type NotifyFuture = NotifyFuture;

    fn call(&mut self, WithRequestId { id, inner: call }: CallWithId) -> CallFuture {
        CallFuture::Begin {
            id,
            call: Some(call),
            dispatch_request_sender: self.take_sender(),
        }
    }

    fn notify(&mut self, notification: NotificationWithId) -> NotifyFuture {
        NotifyFuture {
            id: notification.to_request_id(),
            notification: Some(notification.into_inner()),
            dispatch_request_sender: self.take_sender(),
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub(crate) enum CallFuture {
    Begin {
        id: RequestId,
        call: Option<Call>,
        dispatch_request_sender: PollSender<DispatchRequest>,
    },
    WaitForResponse {
        id: RequestId,
        response_receiver: oneshot::Receiver<Result<Reply, CallTermination<messaging::Error>>>,
    },
    Done {
        id: RequestId,
    },
}

impl ToRequestId for CallFuture {
    fn to_request_id(&self) -> RequestId {
        match self {
            CallFuture::Begin { id, .. }
            | CallFuture::WaitForResponse { id, .. }
            | CallFuture::Done { id } => *id,
        }
    }
}

impl Future for CallFuture {
    type Output = Result<Reply, CallTermination<Error>>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match this {
                Self::Begin {
                    id,
                    call,
                    dispatch_request_sender,
                } => {
                    let call = match call.take() {
                        Some(call) => call,
                        None => return Poll::Pending,
                    };
                    ready!(dispatch_request_sender.poll_reserve(cx))
                        .map_err(|_err| Error::DispatchTerminated)?;
                    let (response_sender, response_receiver) = oneshot::channel();
                    dispatch_request_sender
                        .send_item(DispatchRequest::Call {
                            id: *id,
                            call,
                            response_sender,
                        })
                        .map_err(|_err| Error::DispatchDroppedResponse)?;
                    *this = Self::WaitForResponse {
                        id: *id,
                        response_receiver,
                    };
                }
                Self::WaitForResponse {
                    id,
                    response_receiver,
                } => {
                    let reply = ready!(response_receiver.poll_unpin(cx))
                        .map_err(|_err| Error::DispatchDroppedResponse)?
                        .map_err(|err| err.map_err(Error::Messaging))?;
                    *this = Self::Done { id: *id };
                    break Poll::Ready(Ok(reply));
                }
                Self::Done { .. } => {
                    break Poll::Pending;
                }
            }
        }
    }
}

impl futures::future::FusedFuture for CallFuture {
    fn is_terminated(&self) -> bool {
        matches!(self, Self::Done { .. })
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing until polled"]
pub(crate) struct NotifyFuture {
    id: RequestId,
    notification: Option<Notification>,
    dispatch_request_sender: PollSender<DispatchRequest>,
}

impl ToRequestId for NotifyFuture {
    fn to_request_id(&self) -> RequestId {
        self.id
    }
}

impl Future for NotifyFuture {
    type Output = Result<(), Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let notif = match this.notification.take() {
            Some(notif) => notif,
            None => return Poll::Pending,
        };
        ready!(this.dispatch_request_sender.poll_reserve(cx))
            .map_err(|_err| Error::DispatchTerminated)?;
        this.dispatch_request_sender
            .send_item(DispatchRequest::Notification { id: this.id, notif })
            .map_err(|_err| Error::DispatchTerminated)?;
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("the client dispatch task is terminated")]
    DispatchTerminated,

    #[error("the client dispatch task has dropped the request response")]
    DispatchDroppedResponse,

    #[error(transparent)]
    Messaging(#[from] messaging::Error),
}

async fn dispatch<St, Si>(
    mut request_receiver: mpsc::Receiver<DispatchRequest>,
    requests_sink: Si,
    responses_stream: St,
) -> Result<(), Si::Error>
where
    Si: Sink<RequestWithId>,
    Si::Error: std::error::Error,
    St: Stream<Item = (RequestId, Result<Reply, CallTermination<messaging::Error>>)>,
{
    let mut ongoing_call_requests = HashMap::new();
    let requests_sink = requests_sink;
    let responses_stream = responses_stream.fuse();
    pin!(responses_stream, requests_sink);

    loop {
        select! {
            Some(request) = request_receiver.recv() => {
                let (id, request) = match request {
                    DispatchRequest::Call {
                        id,
                        call,
                        response_sender,
                    } => {
                        debug!(%id, "registering a call request waiting for a response from the server");
                        ongoing_call_requests.insert(id, response_sender);
                        (id, call.into())
                    }
                    DispatchRequest::Notification{ id, notif } => (id, notif.into()),
                };
                requests_sink.send(RequestWithId::new(id, request)).await?;
            }
            Some((id, response)) = responses_stream.next() => {
                debug!(response = ?response, "received a call response from the server");
                if let Some(response_sender) = ongoing_call_requests.remove(&id) {
                    if let Err(response) = response_sender.send(response) {
                        debug!(response = ?response, "the client for a call request response has dropped, discarding response");
                    }
                }
            }
            else => {
                debug!("client dispatch is finished");
                break Ok(());
            }
        }

        // Cleanup ongoing call requests for which the client has dropped the channel.
        ongoing_call_requests.retain(|_id, response_sender| !response_sender.is_closed())
    }
}

#[derive(Debug)]
pub(crate) enum DispatchRequest {
    Call {
        id: RequestId,
        call: Call,
        response_sender: oneshot::Sender<Result<Reply, CallTermination<messaging::Error>>>,
    },
    Notification {
        id: RequestId,
        notif: Notification,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        messaging::{Post, PostWithId, Request, Subject, WithRequestId},
        service,
    };
    use assert_matches::assert_matches;
    use bytes::Bytes;
    use futures::future::{poll_immediate, BoxFuture};
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSendError;

    struct TestClient {
        requests_rx: mpsc::Receiver<RequestWithId>,
        responses_tx: mpsc::Sender<(RequestId, Result<Reply, CallTermination<messaging::Error>>)>,
        client: Client,
        dispatch: BoxFuture<'static, Result<(), PollSendError<RequestWithId>>>,
    }

    impl TestClient {
        fn new() -> Self {
            let (requests_tx, requests_rx) = mpsc::channel(1);
            let (responses_tx, responses_rx) = mpsc::channel(1);
            let requests_sink = PollSender::new(requests_tx);
            let responses_stream = ReceiverStream::new(responses_rx);
            let (client, dispatch) = setup(responses_stream, requests_sink);
            Self {
                requests_rx,
                responses_tx,
                client,
                dispatch: dispatch.boxed(),
            }
        }
    }

    #[tokio::test]
    async fn test_client_drop_client_causes_terminated_error() {
        let mut test = TestClient::new();

        drop(test.dispatch);

        let res = test
            .client
            .call(CallWithId::new(
                RequestId(1),
                Call {
                    subject: Subject::default(),
                    payload: Bytes::from_static(&[1, 2, 3]),
                },
            ))
            .await;
        assert_matches!(res, Err(CallTermination::Error(Error::DispatchTerminated)));

        // Dropping the dispatch between the `ready` and the `call` doesn't make the `call` fail
        // as the `ready` will reserve a slot to send to the dispatch, so the send always succeeds
        // if `ready` succeeds, even if the dispatch is dropped in between.
        //
        // drop(dispatch);
        //
        // let call = service.call(Call {
        //     id: RequestId(1),
        //     subject: Subject::default(),
        //     payload: Bytes::from_static(&[1, 2, 3, 4]),
        // });
        //
        // assert_matches!(
        //     poll_immediate(call).await,
        //     Some(Err(Error::Dispatch(DispatchError::Terminated)))
        // );
    }

    #[tokio::test]
    async fn test_client_drop_dispatch_task_after_call_causes_canceled_error() {
        let mut test = TestClient::new();

        let call = test.client.call(CallWithId::new(
            RequestId(1),
            Call {
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            },
        ));
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The dispatch is now waiting for the response. Drop the task.
        // It must cancel pending requests.
        drop(test.dispatch);
        assert_matches!(
            poll_immediate(call).await,
            Some(Err(CallTermination::Error(Error::DispatchDroppedResponse)))
        );
    }

    #[tokio::test]
    async fn test_client_post() {
        let mut test = TestClient::new();

        let post = test.client.notify(
            PostWithId::new(
                RequestId(1),
                Post {
                    subject: Subject::default(),
                    payload: Bytes::from_static(&[1, 2, 3, 4]),
                },
            )
            .into(),
        );

        pin!(post);

        assert_matches!(poll_immediate(&mut post).await, Some(Ok(())));
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(
                WithRequestId {
                    id: RequestId(1),
                    inner: Request::Notification(
                        Notification::Post(Post {
                            subject,
                            payload,
                        })
                    )
                })) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );
    }

    #[tokio::test]
    async fn test_client_call() {
        let mut test = TestClient::new();

        let call = test.client.call(CallWithId::new(
            RequestId(1),
            Call {
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            },
        ));
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(
                WithRequestId{
                    id: RequestId(1),
                    inner: Request::Call(Call {
                        subject,
                        payload,
                    })
            })) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        test.responses_tx
            .send((
                RequestId(1),
                (Ok(Reply {
                    payload: Bytes::from_static(&[5, 6, 7, 8]),
                })),
            ))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(poll_immediate(&mut call).await, Some(Ok(Reply { payload })) => {
            assert_eq!(payload, Bytes::from_static(&[5, 6, 7, 8]));
        });
    }

    #[tokio::test]
    async fn test_client_call_error() {
        let mut test = TestClient::new();

        let call = test.client.call(CallWithId::new(
            RequestId(1),
            Call {
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            },
        ));
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(
                WithRequestId{
                    id: RequestId(1),
                    inner: Request::Call(Call {
                        subject,
                        payload,
                    })
            })) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        test.responses_tx
            .send((
                RequestId(1),
                Err(CallTermination::Error(messaging::Error(
                    "some error".to_owned(),
                ))),
            ))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(
            poll_immediate(&mut call).await,
            Some(Err(CallTermination::Error(Error::Messaging(service::Error(err))))) => {
                assert_eq!(err, "some error");
            }
        );
    }

    #[tokio::test]
    async fn test_client_call_canceled() {
        let mut test = TestClient::new();

        let call = test.client.call(CallWithId::new(
            RequestId(1),
            Call {
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            },
        ));
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(
                WithRequestId{
                    id: RequestId(1),
                    inner: Request::Call(Call {
                        subject,
                        payload,
                    })
            })) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        test.responses_tx
            .send((RequestId(1), Err(CallTermination::Canceled)))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(
            poll_immediate(&mut call).await,
            Some(Err(CallTermination::Canceled))
        );
    }

    #[tokio::test]
    async fn test_client_call_ignores_responses_of_other_id() {
        let mut test = TestClient::new();

        let call = &mut test.client.call(CallWithId::new(
            RequestId(1),
            Call {
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            },
        ));
        pin!(call);

        assert_matches!(poll_immediate(&mut call).await, None);
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        assert_matches!(
            poll_immediate(test.requests_rx.recv()).await,
            Some(Some(
                WithRequestId{
                    id: RequestId(1),
                    inner: Request::Call(Call {
                        subject,
                        payload,
                    })
            })) => {
                assert_eq!(subject, Subject::default());
                assert_eq!(payload, Bytes::from_static(&[1, 2, 3, 4]));
            }
        );

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        // Send a response of request id = 2.
        test.responses_tx
            .send((
                RequestId(2),
                Ok(Reply {
                    payload: Bytes::from_static(&[5, 6, 7, 8]),
                }),
            ))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call is still waiting for its response.
        assert_matches!(poll_immediate(&mut call).await, None);

        // Send a response of request id = 1.
        test.responses_tx
            .send((
                RequestId(1),
                Ok(Reply {
                    payload: Bytes::from_static(&[9, 10, 11, 12]),
                }),
            ))
            .await
            .unwrap();
        assert_matches!(poll_immediate(&mut test.dispatch).await, None);

        // The call gets its response.
        assert_matches!(poll_immediate(&mut call).await, Some(Ok(Reply { payload })) => {
            assert_eq!(payload, Bytes::from_static(&[9, 10, 11, 12]));
        });
    }

    #[tokio::test]
    async fn test_client_sink_error_stops_dispatch_task() {
        let mut test = TestClient::new();

        // Drop the sink receiver, this will cause errors from the sender.
        drop(test.requests_rx);

        let call = test.client.call(CallWithId::new(
            RequestId(1),
            Call {
                subject: Subject::default(),
                payload: Bytes::from_static(&[1, 2, 3, 4]),
            },
        ));

        // The call cannot finish until dispatch is run.
        assert_matches!(poll_immediate(call).await, None);
        assert_matches!(poll_immediate(test.dispatch).await, Some(Err(_)));
    }
}
