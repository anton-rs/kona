//! This module contains all CLI-specific code.

use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};
use reqwest::Url;

const L1_RPC_URL: &str = "L1_RPC_URL";
const L2_RPC_URL: &str = "L2_RPC_URL";
const BEACON_URL: &str = "BEACON_URL";

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

impl Cli {
    /// Returns the l1 rpc url from CLI or environment variable.
    pub fn l1_rpc_url(&self) -> Result<Url> {
        let url = if let Some(s) = self.l1_rpc_url.clone() {
            s
        } else {
            std::env::var(L1_RPC_URL).map_err(|e| anyhow!(e))?
        };
        Url::parse(&url).map_err(|e| anyhow!(e))
    }

    /// Returns the l2 rpc url from CLI or environment variable.
    pub fn l2_rpc_url(&self) -> Result<Url> {
        let url = if let Some(s) = self.l2_rpc_url.clone() {
            s
        } else {
            std::env::var(L2_RPC_URL).map_err(|e| anyhow!(e))?
        };
        Url::parse(&url).map_err(|e| anyhow!(e))
    }

    /// Returns the beacon url from CLI or environment variable.
    pub fn beacon_url(&self) -> Result<String> {
        Ok(if let Some(s) = self.beacon_url.clone() {
            s
        } else {
            std::env::var(BEACON_URL).map_err(|e| anyhow!(e))?
        })
    }
}
