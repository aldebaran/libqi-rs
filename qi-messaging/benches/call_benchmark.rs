use criterion::{criterion_group, criterion_main, Criterion};
use qi_messaging::{
    call,
    session::{self, Session},
    Action, Object, Service,
};
use std::net::Ipv4Addr;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct ServiceInfo {
    name: String,
    service_id: u32,
    machine_id: String,
    process_id: u32,
    endpoints: Vec<String>,
    session_id: String,
    object_uid: Vec<u8>,
}

async fn run_call(session: &Session) {
    let my_service_info_call = session
        .call(
            call::Params::builder()
                .service(Service::from(1))
                .object(Object::from(1))
                .action(Action::from(100))
                .argument("MyService")
                .build(),
        )
        .unwrap();

    match my_service_info_call.await.unwrap() {
        call::Result::Ok::<ServiceInfo>(_info) => { /* nothing */ }
        call::Result::Err(error) => panic!("{}", error),
        call::Result::Canceled => panic!("the call to ServiceDirectory.service has been canceled"),
    }
}

fn call_benchmark(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let session = runtime.block_on(async {
        let tcp_stream = TcpStream::connect((Ipv4Addr::LOCALHOST, 9559))
            .await
            .unwrap();
        let (session, connect) = session::connect(tcp_stream);
        tokio::spawn(async move {
            if let Err(err) = connect.await {
                tracing::error!("connection error: {err}");
            }
        });

        session.await.unwrap()
    });

    c.bench_function("call", |b| {
        b.to_async(&runtime).iter(|| run_call(&session));
    });
}

criterion_group! { benches, call_benchmark }
criterion_main! { benches }
