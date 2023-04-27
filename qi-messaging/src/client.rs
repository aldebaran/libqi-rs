pub mod builder {
    use crate::{
        message::{Decoder, Encoder, Message},
        server::MessageBuilderExt,
        service::{
            client::{self, Client},
            Service,
        },
        session::{Error, Session},
    };
    use futures::{Sink, TryStream};
    use tokio::io::{split, AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
    use tokio_util::codec::{FramedRead, FramedWrite};

    #[derive(Debug)]
    pub struct Builder<Si, St, Svc> {
        sink: Si,
        stream: St,
        service: Svc,
    }

    impl Builder<Unset, Unset, Unset> {
        pub fn new() -> Self {
            Self
        }
    }

    impl<Svc> Builder<Unset, Unset, Svc> {
        pub fn over_io<IO>(
            self,
            io: IO,
        ) -> Builder<
            Set<FramedRead<ReadHalf<IO>, Decoder>>,
            Set<FramedWrite<WriteHalf<IO>, Encoder>>,
            Svc,
        >
        where
            IO: AsyncWrite + AsyncRead,
        {
            let (input, output) = split(io);
            self.over_in_out(input, output)
        }

        pub fn over_in_out<I, O>(
            self,
            input: I,
            output: O,
        ) -> Builder<Set<FramedRead<I, Decoder>>, Set<FramedWrite<O, Encoder>>, Svc>
        where
            I: AsyncRead,
            O: AsyncWrite,
        {
            let stream = FramedRead::new(input, Decoder::new());
            let sink = FramedWrite::new(output, Encoder);
            self.over_sink_stream(sink, stream)
        }

        pub fn over_sink_stream<Si, St, E, DE>(
            self,
            sink: Si,
            stream: St,
        ) -> Builder<Set<Si>, Set<St>, Svc>
        where
            Si: Sink<Message>,
            St: TryStream<Ok = Message>,
        {
            Builder {
                sink: Set(sink),
                stream: Set(stream),
                service: self.service,
            }
        }
    }

    impl<Si, St> Builder<Si, St, Unset> {
        pub fn serve<Svc>(self, service: Svc) -> Builder<Si, St, Set<Svc>>
        where
            Svc: Service,
        {
            Builder {
                sink: self.sink,
                stream: self.stream,
                service: Set(service),
            }
        }
    }

    impl<Si, St, Svc> Builder<Set<Si>, Set<St>, Set<Svc>>
    where
        Si: Sink<Message>,
        St: TryStream<Ok = Message>,
        Svc: Service,
    {
        pub fn connect(self) -> (Session, impl Future<Result<Session, ConnectError>>) {
            let (input, output) = split(io);
            let input = FramedRead::new(Box::pin(input), Codec::new());
            let output = FramedWrite::new(Box::pin(output), Codec::new());
            let id_gen = IdGenerator::new();
            let mut capabilities = capabilities::local();

            let (authentication_sender, authentication_receiver) = oneshot::channel();
            let client_handler = handlers.add_handler(Client::new(authentication_sender));
            message_sender
                .send(
                    Message::builder()
                        .server_authenticate(id_gen.generate(), &capabilities)
                        .expect("failed to serialize local message capabilities"),
                )
                .await?;

            let remote_capabilities = match authentication_receiver
                .await
                .map_err(|_err| ConnectError::AuthenticationResponseNotReceived)?
            {
                Ok(capabilities) => capabilities,
                Err(_) => todo!(),
            };

            capabilities
                .resolve_minimums_against(&remote_capabilities, capabilities::reset_to_default);
            Ok(Session {
                sender: message_sender,
                id_gen,
                handlers: handlers.downgrade(),
                client_handler,
                capabilities,
            })
        }
    }

    #[derive(Debug)]
    pub struct Set<T>(T);

    #[derive(Debug)]
    pub struct Unset;
}

pub use builder::Builder;

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {}
