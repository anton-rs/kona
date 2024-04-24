mod parser;
mod types;
use crate::{
    parser::parse_b256,
    types::{Network, RpcKind}
};

use alloy_primitives::B256;
use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};
use serde::Serialize;
use std::path::PathBuf;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = Cli::parse();
    let _ = init_tracing_subscriber(_cli.v);
    tracing::info!("host telemetry initialized");
    Ok(())
}

/// Initializes the tracing subscriber
///
/// # Arguments
/// * `verbosity_level` - The verbosity level (0-4)
///
/// # Returns
/// * `Result<()>` - Ok if successful, Err otherwise.
fn init_tracing_subscriber(verbosity_level: u8) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(match verbosity_level {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber).map_err(|e| anyhow!(e))
}

/// The host binary CLI application arguments.
#[derive(Parser, Serialize)]
pub struct Cli {
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    v: u8,
    /// The rollup chain parameters
    #[clap(long)]
    pub rollup_config: PathBuf,
    /// Predefined network selection.
    #[clap(long)]
    pub network: Network,
    /// The Data Directory for preimage data storage. Default uses in-memory storage.
    #[clap(long)]
    pub data_dir: Option<PathBuf>,
    /// Address of L2 JSON-RPC endpoint to use (eth and debug namespace required).
    #[clap(long)]
    pub l2_node_address: String,
    /// Hash of the L1 head block. Derivation stops after this block is processed.
    #[clap(long, value_parser = parse_b256)]
    pub l1_head: B256,
    /// Hash of the L2 block at the L2 Output Root.
    #[clap(long, value_parser = parse_b256)]
    pub l2_head: B256,
    /// Agreed L2 Output Root to start derivation from.
    #[clap(long, value_parser = parse_b256)]
    pub l2_output_root: B256,
    /// Claimed L2 output root to validate
    #[clap(long, value_parser = parse_b256)]
    pub l2_claim: B256,
    /// Number of the L2 block that the claim is from.
    #[clap(long)]
    pub l2_block_number: u64,
    //// Path to the genesis file.
    #[clap(long)]
    pub l2_genesis_path: PathBuf,
    /// Address of L1 JSON-RPC endpoint to use (eth namespace required)
    #[clap(long)]
    pub l1_node_address: String,
    /// Address of the L1 Beacon API endpoint to use.
    #[clap(long)]
    pub l1_beacon_address: String,
    /// Trust the L1 RPC, sync faster at risk of malicious/buggy RPC providing bad or inconsistent
    /// L1 data
    #[clap(long)]
    pub l1_trust_rpc: bool,
    /// The kind of RPC provider, used to inform optimal transactions receipts fetching, and thus
    /// reduce costs.
    #[clap(long)]
    pub l1_rpc_provider_kind: RpcKind,
    /// Run the specified client program as a separate process detached from the host. Default is
    /// to run the client program in the host process.
    #[clap(long)]
    pub exec: String,
    /// Run in pre-image server mode without executing any client program.
    #[clap(long)]
    pub server: bool,
}
