use crate::cli::{init_tracing_subscriber, HostCli};
use anyhow::Result;
use clap::Parser;

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let HostCli { v: tracing_verbosity, .. } = HostCli::parse();
    let _ = init_tracing_subscriber(tracing_verbosity);
    tracing::info!("host telemetry initialized");
    Ok(())
}
