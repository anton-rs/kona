use clap::ValueEnum;
use serde::Serialize;

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