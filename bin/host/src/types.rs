use clap::ValueEnum;
use serde::Serialize;

/// Available networks.
#[derive(Debug, Clone, ValueEnum, Serialize)]
pub enum Network {
    /// Optimism Mainnet
    Optimism,
}

/// Available RPC provider types.
#[derive(Debug, Clone, ValueEnum, Serialize)]
pub enum RpcKind {
    /// debug alloy provider
    DebugRpc,
}