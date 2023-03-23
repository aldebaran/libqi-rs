use qi_messaging::Session;
use tokio::net::TcpStream;
use std::net::Ipv4Addr;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let tcp_stream = TcpStream::connect(((Ipv4Addr::LOCALHOST, 9559)).await?;
    let _session = Session::connect(tcp_stream).await?;

    Ok(())
}
