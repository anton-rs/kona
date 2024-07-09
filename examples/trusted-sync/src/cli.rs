//! This module contains all CLI-specific code.

use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};
use reqwest::Url;

const L1_RPC_URL: &str = "L1_RPC_URL";
const L2_RPC_URL: &str = "L2_RPC_URL";
const BEACON_URL: &str = "BEACON_URL";
const METRICS_URL: &str = "METRICS_URL";
const DEFAULT_METRICS_SERVER_ADDR: &str = "0.0.0.0";
const DEFAULT_METRICS_SERVER_PORT: u16 = 9000;
const DEFAULT_LOKI_SERVER_ADDR: &str = "0.0.0.0";
const DEFAULT_LOKI_SERVER_PORT: u16 = 3100;

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
    /// The url for metrics.
    #[clap(long, help = "Address of the metrics server")]
    pub metrics_url: Option<String>,
    /// The Loki Url endpoint.
    #[clap(long, help = "Url to post Loki logs")]
    pub loki_url: Option<String>,
    /// Whether to enable Loki Metrics.
    #[clap(long, help = "Enable Loki metrics")]
    pub loki_metrics: bool,
    /// Start blocks from tip.
    #[clap(long, help = "Number of blocks prior to tip to start from")]
    pub start_blocks_from_tip: Option<u64>,
}

impl Cli {
    /// Returns the full metrics server address string.
    pub fn metrics_server_addr(&self) -> String {
        if let Some(url) = self.metrics_url.clone() {
            return url;
        }
        if let Ok(url) = std::env::var(METRICS_URL) {
            return url;
        }
        format!("{}:{}", DEFAULT_METRICS_SERVER_ADDR, DEFAULT_METRICS_SERVER_PORT)
    }

    /// Returns the full loki server address.
    pub fn loki_addr(&self) -> Url {
        if let Some(url) = self.loki_url.clone() {
            return Url::parse(&url).expect("Failed to parse loki server address");
        }
        let str = format!("http://{DEFAULT_LOKI_SERVER_ADDR}:{DEFAULT_LOKI_SERVER_PORT}");
        Url::parse(&str).expect("Failed to parse loki server address")
    }

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
