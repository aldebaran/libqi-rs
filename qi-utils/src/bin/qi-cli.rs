use anyhow::{bail, Result};
use iri_string::types::UriString;
use qi_messaging::{session, Action, CallResult, Object, Params, Service};
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
    let (session, connect) = session::connect(tcp_stream);

    tokio::spawn(async move {
        if let Err(err) = connect.await {
            tracing::error!("connection error: {err}");
        }
    });

    let session = session.await?;

    let my_service_info = session
        .call(
            Params::builder()
                .service(Service::from(1))
                .object(Object::from(1))
                .action(Action::from(100))
                .argument("MyService")
                .build(),
        )
        .await?;

    match my_service_info.await? {
        CallResult::Ok::<ServiceInfo>(info) => {
            println!("MyService: {info:?}");
        }
        CallResult::Err(error) => bail!(error),
        CallResult::Canceled => bail!("the call to ServiceDirectory.service has been canceled"),
    };

    let services = session
        .call(
            Params::builder()
                .service(Service::from(1))
                .object(Object::from(1))
                .action(Action::from(101))
                .argument(())
                .build(),
        )
        .await?;
    let _services = match services.await? {
        CallResult::Ok::<Vec<ServiceInfo>>(services) => {
            println!("services: {services:?}");
            services
        }
        CallResult::Err(error) => bail!(error),
        CallResult::Canceled => bail!("the call to ServiceDirectory.services has been canceled"),
    };

    Ok(())
}
