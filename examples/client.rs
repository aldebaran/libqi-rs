use clap::Parser;
use eyre::Result;
use qi::{Address, ObjectExt};
use tracing::{debug_span, info, Instrument};
use tracing_subscriber::fmt;

#[derive(Debug, clap::Parser)]
#[clap()]
struct Args {
    #[clap(short, long, default_value = "tcp://localhost:9559")]
    address: Address,

    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Activate traces to the console.
    tracing_subscriber::fmt()
        .compact()
        .with_max_level(match args.verbose {
            0 => Some(tracing::Level::WARN),
            1 => Some(tracing::Level::INFO),
            2 => Some(tracing::Level::DEBUG),
            3.. => Some(tracing::Level::TRACE),
        })
        .with_target(false)
        .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    // Create a node and spawn its task inside the runtime's executor.
    let (node, task) = qi::node::open();
    tokio::spawn(task.instrument(debug_span!("node task", address = %args.address)));

    // Connect the node to a space at the given address.
    info!(address = %args.address, "connecting node to space");
    let node = node
        .connect_to_space(qi::session::Config::default().add_addresses([args.address.clone()]))
        .await?;

    // You can add services to the node and make them accessible to other nodes of joined spaces.
    // TODO

    // You can access remote services and call methods on them.
    info!("getting \"Calculator\" service");
    let calculator = node.service("Calculator").await?;
    calculator.call("reset", 3).await?; // => 3
    calculator.call("add", 9).await?; // => 12
    calculator.call("mul", 4).await?; // => 48
    calculator.call("add", 80).await?; // => 128
    calculator.call("div", 2).await?; // => 64
    let result: i32 = calculator.call("ans", ()).await?;
    info!(%result, "calculation is done"); // result = 64

    // You can send local objects to remote nodes for them to call methods on.
    // TODO

    Ok(())
}