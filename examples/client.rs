use clap::Parser;
use eyre::Result;
use qi::services::{Address, ObjectExt};
use tracing::{debug_span, info, Instrument};
use tracing_subscriber::fmt;

#[derive(Debug, clap::Parser)]
#[clap()]
struct Args {
    #[clap(short, long, default_value = "tcp://localhost:9559")]
    url: Address,

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

    // Open a node and spawn its task inside the runtime's executor.
    info!("opening node");
    let (node, task) = qi::Node::open();
    tokio::spawn(task.instrument(debug_span!("node task")));

    // You can register services and make them accessible to other nodes.
    // TODO
    info!(url = %args.url, "connecting to space");
    let space = node.connect_to_space([args.url], None).await?;

    // You can access remote services and call methods on them.
    info!("getting \"Calculator\" service");
    let calculator = space.service("Calculator").await?;
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
