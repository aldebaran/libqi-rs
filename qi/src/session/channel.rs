use crate::{messaging, session::address::SslKind, Address};
use bytes::Bytes;
use futures::{
    stream::{self, FusedStream},
    SinkExt, Stream, TryStream,
};
use messaging::{endpoint::OutgoingMessages, message};
use std::{
    marker::PhantomData,
    pin::Pin,
    task::{ready, Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::{FramedRead, FramedWrite};

pub(crate) async fn connect<'a>(
    address: Address,
) -> Result<(impl AsyncRead, impl AsyncWrite), std::io::Error> {
    match address {
        Address::Tcp { address, ssl: None } => Ok(TcpStream::connect(address).await?.into_split()),
        _ => unimplemented!(),
    }
}

fn rw_messaging_endpoint<Read, Write, Svc, Sink, ReadBody, WriteBody>(
    read: Read,
    write: Write,
    service: Svc,
    oneway_request_sink: Sink,
) -> (messaging::Client<WriteBody, ReadBody>, Connection)
where
    Read: AsyncRead,
    Write: AsyncWrite,
    Svc: tower::Service<(message::Address, WriteBody)>,
    WriteBody: messaging::BodyBuf + Send,
{
    let messages_stream = FramedRead::new(read, messaging::binary_codec::Decoder::new());
    let messages_sink = FramedWrite::new(write, messaging::binary_codec::Encoder);

    let (client, outgoing) = messaging::endpoint(messages_stream, service, oneway_request_sink);
    let connection = Connection {
        inner: outgoing.forward(messages_sink),
    };
    (client, connection)
}

pub(crate) async fn serve<'a, F, Svc>(
    address: Address,
    make_service: F,
) -> Result<(ServerClientsStream<'a, F>, Vec<Address>), std::io::Error>
where
    F: FnMut() -> Svc,
    Svc: tower::Service<(message::Address, Bytes)> + 'a,
{
    match address {
        Address::Tcp { address, ssl } => {
            if address.port() == 0 {
                unimplemented!("binding to a TCP endpoint with port 0 is not yet supported")
            }
            if ssl.is_some() {
                unimplemented!("binding to a TCP endpoint with SSL is not yet supported")
            }
            let listener = TcpListener::bind(address).await?;
            let endpoints = listener
                .local_addr()
                .map(|address| Address::Tcp { address, ssl })
                .into_iter()
                .collect();
            let clients = ServerClientsStream {
                listener,
                make_service,
                ssl,
                phantom: PhantomData,
            };
            Ok((clients, endpoints))
        }
    }
}

#[derive(Debug)]
pub(crate) struct Connection<Msgs, Svc, Snk, ReadBody, WriteBody, Write>
where
    Msgs: TryStream<Ok = messaging::Message<ReadBody>>,
{
    inner: stream::Forward<
        OutgoingMessages<Msgs, Svc, Snk, ReadBody, WriteBody>,
        FramedWrite<Write, messaging::binary_codec::Encoder>,
    >,
}

pub(crate) struct ServerClientsStream<'a, F> {
    listener: TcpListener,
    make_service: F,
    ssl: Option<SslKind>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, F, Svc> Stream for ServerClientsStream<'a, F>
where
    F: FnMut() -> Svc,
    Svc: tower::Service<(message::Address, Bytes)> + 'a,
{
    type Item = (messaging::Client, Connection<'a>, Address);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // TODO: Handle case when accept returns an error that is fatal for this listener.
            if let Ok((socket, address)) = ready!(self.listener.poll_accept(cx)) {
                let (read, write) = socket.into_split();
                let (client, connection) =
                    rw_messaging_endpoint(read, write, (self.make_service)());
                return Poll::Ready(Some((
                    client,
                    connection,
                    Address::Tcp {
                        address,
                        ssl: self.ssl,
                    },
                )));
            }
        }
    }
}

impl<'a, F> FusedStream for ServerClientsStream<'a, F>
where
    Self: Stream,
{
    fn is_terminated(&self) -> bool {
        false
    }
}
