use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use qi_messaging::{
    call,
    session::{self, Session},
    Action, MessageCodec, Object, Service, Type,
};
use tokio::{io::duplex, join, runtime::Runtime};
use tokio_util::codec::Framed;

const MAX_BUFFER_SIZE: usize = 256 * 1024;

async fn execute_call(session: &Session, payload_size: usize) {
    let my_service_info_call = session
        .call(
            call::Params::builder()
                .service(Service::from(1))
                .object(Object::from(2))
                .action(Action::from(3))
                .argument(vec![0; payload_size])
                .build(),
        )
        .unwrap();

    match my_service_info_call.await.unwrap() {
        call::Result::Ok::<Vec<u8>>(_buf) => { /* nothing */ }
        call::Result::Err(error) => panic!("{}", error),
        call::Result::Canceled => panic!("canceled"),
    }
}

fn dispatch_benchmark(c: &mut Criterion) {
    let (client, server) = duplex(MAX_BUFFER_SIZE);
    let runtime = Runtime::new().unwrap();
    let (session, connect) = session::connect(client);

    let connect = runtime.spawn(connect);
    let server = runtime.spawn(async move {
        use futures::{SinkExt, StreamExt};
        let mut server = Framed::new(server, MessageCodec);
        while let Some(msg) = server.next().await {
            let mut msg = msg.unwrap();
            assert_eq!(msg.ty, Type::Call);
            msg.ty = Type::Reply;
            if let Err(err) = server.send(msg).await {
                panic!("server send reply error: {}", err);
            }
        }
    });
    let session = runtime.block_on(async move { session.await.unwrap() });

    let mut group = c.benchmark_group("dispatch call with increasing payload size");
    for power in (0..=16).step_by(4) {
        let size = 2usize.pow(power);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&runtime).iter(|| execute_call(&session, size));
        });
    }
    group.finish();

    drop(session);
    let (f1, f2) = runtime.block_on(async move { join!(server, connect) });
    f1.unwrap();
    f2.unwrap().unwrap();
}

criterion_group! { benches, dispatch_benchmark }
criterion_main! { benches }
