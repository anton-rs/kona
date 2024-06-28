#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use crate::cli::init_tracing_subscriber;
use anyhow::Result;
use clap::Parser;
use tracing::info;

// use tokio::{process::Command, sync::RwLock, task};

use kona_host::{start_server, start_server_and_native_client, HostCli};

mod cli;
mod fetcher;
mod kv;
mod server;
mod types;
mod util;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cfg = HostCli::parse();
    init_tracing_subscriber(cfg.v)?;

    if cfg.server {
        start_server(cfg).await?;
    } else {
        start_server_and_native_client(cfg).await?;
    }

    info!("Exiting host program.");
    Ok(())
}
