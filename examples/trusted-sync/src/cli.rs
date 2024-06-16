//! This module contains all CLI-specific code.

use clap::{ArgAction, Parser};

/// The host binary CLI application arguments.
#[derive(Parser, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cli {
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    pub v: u8,
    /// The l1 rpc URL
    #[clap(long, short = '1')]
    pub l1_rpc_url: Option<String>,
    /// The l2 rpc URL
    #[clap(long, short = '2')]
    pub l2_rpc_url: Option<String>,
    /// The Beacon URL
    #[clap(long, short)]
    pub beacon_url: Option<String>,
    /// The l2 block to start from.
    #[clap(long, short, help = "Starting l2 block, defaults to chain genesis if none specified")]
    pub start_l2_block: Option<u64>,
}
