use criterion::{criterion_group, criterion_main, Criterion};
use qi_messaging::{client, Action, Object, Response, Service, Session};
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

async fn run_call(client: &client::Client) {
    let my_service_info_response = client
        .call()
        .service(Service::from(1))
        .object(Object::from(1))
        .action(Action::from(100))
        .argument("MyService")
        .send()
        .unwrap();

    match my_service_info_response.await.unwrap() {
        Response::Reply(reply) => {
            let _info: ServiceInfo = reply.into_value();
        }
        Response::Error(error) => panic!("{}", error.into_description()),
        Response::Canceled(_) => panic!("the call to ServiceDirectory.service has been canceled"),
    }
}

fn call_benchmark(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let client = runtime.block_on(async {
        let tcp_stream = TcpStream::connect((Ipv4Addr::LOCALHOST, 9559))
            .await
            .unwrap();
        let (client, connect) = client::connect(tcp_stream);
        tokio::spawn(async move {
            if let Err(err) = connect.await {
                tracing::error!("connection error: {err}");
            }
        });

        client.await.unwrap()
    });

    c.bench_function("call", |b| {
        b.to_async(&runtime).iter(|| run_call(&client));
    });
}

criterion_group! { benches, call_benchmark }
criterion_main! { benches }
