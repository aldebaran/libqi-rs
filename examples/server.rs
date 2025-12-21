mod audio;
mod config;

use self::config::{Args, UserAndToken};
use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Wait for interruption
    let interrupt = tokio::spawn(async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {}", err);
                // we also shut down in case of error
            }
        }
    });

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
    let node_builder = qi::node::Builder::new()
        // You can add services to the node and make them accessible to other nodes of joined spaces.
        .add_service("AudioPlayer", AudioPlayer::new())
        // Host the space on this node
        .bind(args.address)
        .host_space();

    if let Some(UserAndToken { user, token }) = args.user_and_token {
        node_builder.with_authenticator(qi::auth::UserTokenAuthenticator::new(user, token));
    }

    let _node = node_builder
        .start()
        .await
        .with_context(|| format!("Failed to host space for node at address {}", args.address))?;

    let _res = interrupt.await;
    Ok(())
}

#[derive(Default, Debug)]
pub(crate) struct AudioPlayer;

impl AudioPlayer {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl audio::Player for AudioPlayer {}
