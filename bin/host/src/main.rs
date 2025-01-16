//! Main entrypoint for the host binary.

use anyhow::Result;
use clap::Parser;
use kona_host::{cli::HostMode, init_tracing_subscriber, HostCli};
use tracing::info;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cfg = HostCli::parse();
    init_tracing_subscriber(cfg.v)?;

    match cfg.mode {
        HostMode::Single(cfg) => {
            cfg.run().await?;
        }
        HostMode::Super(cfg) => {
            cfg.run().await?;
        }
    }

    info!("Exiting host program.");
    Ok(())
}
