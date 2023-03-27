use crate::dispatch;
use tokio::{io::AsyncRead, io::AsyncWrite};

#[derive(derive_new::new, Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Connection<IO> {
    dispatch: dispatch::Dispatch<IO>,
}

impl<IO> std::future::Future for Connection<IO>
where
    IO: AsyncRead + AsyncWrite,
{
    type Output = Result<(), ConnectionError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.dispatch.poll_dispatch(cx)
    }
}

pub use dispatch::DispatchError as ConnectionError;
