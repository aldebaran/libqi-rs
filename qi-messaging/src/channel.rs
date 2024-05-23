use crate::{address::Address, binary_codec, endpoint, message, BodyBuf, Client, Error};
use futures::{Sink, SinkExt, StreamExt};
use std::{future::Future, pin::Pin};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

pub async fn connect<'a, WriteBody, ReadBody, Handler, Snk>(
    address: Address,
    handler: Handler,
    oneway_requests_sink: Snk,
) -> Result<
    (
        Client<WriteBody, ReadBody>,
        impl Future<Output = Result<(), Error>>,
    ),
    std::io::Error,
>
where
    Handler: tower_service::Service<(message::Address, ReadBody), Response = WriteBody>,
    Handler::Error: std::string::ToString + Into<Box<dyn std::error::Error + Send + Sync>>,
    Snk: Sink<(message::Address, message::OnewayRequest<ReadBody>)>,
    Snk::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    ReadBody: BodyBuf + Send,
    ReadBody::Error: std::error::Error + Sync + Send + 'static,
    WriteBody: BodyBuf + Send,
    WriteBody::Error: std::error::Error + Send + Sync + 'static,
{
    let (read, write) = connect_transport(address).await?;
    let messages_stream = FramedRead::new(read, binary_codec::Decoder::new());
    let messages_sink = FramedWrite::new(write, binary_codec::Encoder);

    let (client, outgoing_messages) = endpoint(messages_stream, handler, oneway_requests_sink);
    let connection = outgoing_messages.forward(messages_sink.sink_err_into());
    Ok((client, connection))
}

async fn connect_transport(
    address: Address,
) -> Result<
    (
        Pin<Box<dyn AsyncRead + Send>>,
        Pin<Box<dyn AsyncWrite + Send>>,
    ),
    std::io::Error,
> {
    match address {
        Address::Tcp { address, ssl: None } => {
            let (read, write) = TcpStream::connect(address).await?.into_split();
            Ok((Box::pin(read), Box::pin(write)))
        }
        _ => unimplemented!(),
    }
}

// pub(crate) async fn serve<'a, F, Svc>(
//     address: Address,
//     make_service: F,
// ) -> Result<(ServerClientsStream<'a, F>, Vec<Address>), std::io::Error>
// where
//     F: FnMut() -> Svc,
//     Svc: tower_service::Service<(message::Address, Bytes)> + 'a,
// {
//     match address {
//         Address::Tcp { address, ssl } => {
//             if address.port() == 0 {
//                 unimplemented!("binding to a TCP endpoint with port 0 is not yet supported")
//             }
//             if ssl.is_some() {
//                 unimplemented!("binding to a TCP endpoint with SSL is not yet supported")
//             }
//             let listener = TcpListener::bind(address).await?;
//             let endpoints = listener
//                 .local_addr()
//                 .map(|address| Address::Tcp { address, ssl })
//                 .into_iter()
//                 .collect();
//             let clients = ServerClientsStream {
//                 listener,
//                 make_service,
//                 ssl,
//                 phantom: PhantomData,
//             };
//             Ok((clients, endpoints))
//         }
//     }
// }

// pub(crate) struct ServerClientsStream<'a, F> {
//     listener: TcpListener,
//     make_service: F,
//     ssl: Option<SslKind>,
//     phantom: PhantomData<&'a ()>,
// }

// impl<'a, F, Svc> Stream for ServerClientsStream<'a, F>
// where
//     F: FnMut() -> Svc,
//     Svc: tower_service::Service<(message::Address, Bytes)> + 'a,
// {
//     type Item = (messaging::Client, Connection<'a>, Address);

//     fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         loop {
//             // TODO: Handle case when accept returns an error that is fatal for this listener.
//             if let Ok((socket, address)) = ready!(self.listener.poll_accept(cx)) {
//                 let (read, write) = socket.into_split();
//                 let (client, connection) = connect_rw_endpoint(read, write, (self.make_service)());
//                 return Poll::Ready(Some((
//                     client,
//                     connection,
//                     Address::Tcp {
//                         address,
//                         ssl: self.ssl,
//                     },
//                 )));
//             }
//         }
//     }
// }

// impl<'a, F> FusedStream for ServerClientsStream<'a, F>
// where
//     Self: Stream,
// {
//     fn is_terminated(&self) -> bool {
//         false
//     }
// }
