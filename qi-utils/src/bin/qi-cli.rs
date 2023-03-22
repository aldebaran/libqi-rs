use qi_messaging::Channel;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let tcp_stream = TcpStream::connect("localhost:9559").await?;
    let _session = Channel::new(tcp_stream).await?;

    Ok(())
}
