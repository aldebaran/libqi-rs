use anyhow::{bail, Result};
use iri_string::types::UriString;
use qi_messaging::{client, Action, Object, Response, Service, Session};
use std::net::Ipv4Addr;
use tokio::net::TcpStream;

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct ServiceInfo {
    name: String,
    service_id: u32,
    machine_id: String,
    process_id: u32,
    endpoints: Vec<UriString>,
    session_id: String,
    object_uid: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(tracing::Level::TRACE)
        .with_thread_ids(true)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let tcp_stream = TcpStream::connect((Ipv4Addr::LOCALHOST, 9559)).await?;
    let (client, connect) = client::connect(tcp_stream);

    tokio::spawn(async move {
        if let Err(err) = connect.await {
            tracing::error!("connection error: {err}");
        }
    });

    let client = client.await?;

    let my_service_info_response = client
        .call()
        .service(Service::from(1))
        .object(Object::from(1))
        .action(Action::from(100))
        .argument("MyService")
        .send()?;

    match my_service_info_response.await? {
        Response::Reply(reply) => {
            let info: ServiceInfo = reply.into_value();
            println!("MyService: {info:?}");
            Ok(())
        }
        Response::Error(error) => bail!(error.into_description()),
        Response::Canceled(_) => bail!("the call to ServiceDirectory.service has been canceled"),
    }
}
