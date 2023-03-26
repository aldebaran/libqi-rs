use crate::message::Message;
use sync::{
    mpsc::{self, error::SendError},
    oneshot,
};
pub(crate) use tokio::sync;
use tracing::trace;

pub(crate) fn request_channel() -> (RequestSender, RequestReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (RequestSender(tx), rx)
}

#[derive(Debug)]
pub(crate) enum Request {
    CallRequest {
        msg: Message,
        resp_tx: ResponseSender,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct RequestSender(mpsc::UnboundedSender<Request>);

impl RequestSender {
    pub(crate) fn send_call_request(&self, msg: Message) -> Result<ResponseReceiver, Message> {
        let (resp_tx, resp_rx) = oneshot::channel();
        match self.0.send(Request::CallRequest {
            msg,
            resp_tx: ResponseSender(resp_tx),
        }) {
            Ok(()) => {}
            Err(SendError(dispatch)) => match dispatch {
                Request::CallRequest { msg, .. } => return Err(msg),
            },
        }
        Ok(resp_rx)
    }
}

pub(crate) type RequestReceiver = mpsc::UnboundedReceiver<Request>;

pub(crate) type Response = Result<Message, ResponseError>;

#[derive(Debug)]
pub(crate) struct ResponseSender(oneshot::Sender<Response>);

impl ResponseSender {
    pub(crate) fn is_closed(&self) -> bool {
        self.0.is_closed()
    }

    pub(crate) fn send(self, resp: Response) {
        if self.0.send(resp).is_err() {
            trace!("a response receiver dropped, the response is discarded");
        }
    }
}

pub(crate) type ResponseReceiver = oneshot::Receiver<Response>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ResponseError {
    #[error("a request with the same ID already exists")]
    RequestIdAlreadyExists,
}
