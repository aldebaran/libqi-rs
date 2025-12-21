mod args;
mod audio;

use anyhow::{Context, Result};
use args::Args;
use clap::Parser;
use qi::ObjectExt;
use tracing::info;
use tracing_subscriber::fmt;

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

    info!("creating node");
    let node = qi::node::Builder::new()
        .connect_to_space(args.address, None)
        .start()
        .await
        .with_context(|| {
            format!(
                "Failed to connect node to space at address {}",
                args.address
            )
        })?;

    // You can access remote services and call methods on them.
    info!("getting \"Calculator\" service");
    let calculator = node.service("Calculator").await?;
    let () = calculator.call("reset", 3).await?; // => 3
    let () = calculator.call("add", 9).await?; // => 12
    let () = calculator.call("mul", 4).await?; // => 48
    let () = calculator.call("add", 80).await?; // => 128
    let () = calculator.call("div", 2).await?; // => 64
    let result: i32 = calculator.call("ans", ()).await?;
    info!(%result, "calculation is done"); // result = 64

    Ok(())
}
