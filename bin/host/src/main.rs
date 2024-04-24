use alloy_primitives::B256;
use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    tracing_subscriber::Registry::default().init();
    tracing::info!("host telemetry initialized");
    Ok(())
}

fn parse_b256(s: &str) -> Result<B256, String> {
    B256::from_str(s).map_err(|_| format!("Invalid B256 value: {}", s))
}

/// Available networks.
#[derive(Debug, Clone, ValueEnum, Serialize)]
pub enum Network {
    /// Optimism Mainnet
    Optimism,
}

#[derive(Debug, Clone, ValueEnum, Serialize)]
pub enum RpcKind {
    DebugRpc,
}

/// The host binary CLI application arguments.
#[derive(Parser, Serialize)]
pub struct Cli {
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
    /// Trust the L1 RPC, sync faster at risk of malicious/buggy RPC providing bad or inconsistent L1 data
    #[clap(long)]
    pub l1_trust_rpc: bool,
    /// The kind of RPC provider, used to inform optimal transactions receipts fetching, and thus reduce costs.
    #[clap(long)]
    pub l1_rpc_provider_kind: RpcKind,
    /// Run the specified client program as a separate process detached from the host. Default is to run the client program in the host process.
    #[clap(long)]
    pub exec: String,
    /// Run in pre-image server mode without executing any client program.
    #[clap(long)]
    pub server: bool,
}
