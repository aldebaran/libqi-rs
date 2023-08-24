#![deny(unsafe_code)]
#![warn(unused_crate_dependencies)]

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// URI to the namespace to connect to.
    #[clap(short, long, default_value = "tcp://localhost:9559")]
    uri: qi::Uri,

    #[clap(short, long)]
    verbose: bool,
}

async fn print_service(service: &qi::ServiceInfo, details: bool) -> Result<()> {
    const INDENT: &str = "";
    println!(
        "{id:0>3} [{name}]",
        id = format!("{}", service.service_id).magenta(),
        name = service.name.red(),
    );
    if !details {
        return Ok(());
    }
    println!(
        "{INDENT:level$}{} {}",
        "*".green(),
        "Info".magenta(),
        level = 2
    );
    println!(
        "{INDENT:level$}{} {}",
        "machine".bold(),
        service.machine_id,
        level = 4
    );
    println!(
        "{INDENT:level$}{} {}",
        "process".bold(),
        service.process_id,
        level = 4
    );
    println!("{INDENT:level$}{}", "endpoints".bold(), level = 4);
    for endpoint in &service.endpoints {
        println!("{INDENT:level$}- {}", endpoint, level = 6);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Activate traces to the console.
    if args.verbose {
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_max_level(tracing::Level::TRACE)
            .with_thread_ids(true)
            .with_target(true)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
    }

    let node = qi::Node::to_namespace(args.uri).await?;
    let service_directory = node.service_directory();
    let services = service_directory.services().await?;

    for service in services {
        print_service(&service, true).await?;
    }

    Ok(())
}
