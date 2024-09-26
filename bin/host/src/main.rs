//! Main entrypoint for the host binary.

use anyhow::Result;
use clap::Parser;
use kona_host::{init_tracing_subscriber, start_server, start_server_and_native_client, HostCli};
use tracing::{error, info};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cfg = HostCli::parse();
    init_tracing_subscriber(cfg.v)?;

    if cfg.server {
        start_server(cfg).await?;
    } else {
        let status = match start_server_and_native_client(cfg).await {
            Ok(status) => status,
            Err(e) => {
                error!(target: "kona_host", "Exited with an error: {:?}", e);
                panic!("{e}");
            }
        };

        // Bubble up the exit status of the client program.
        std::process::exit(status as i32);
    }

    info!("Exiting host program.");
    Ok(())
}
