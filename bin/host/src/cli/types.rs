use clap::ValueEnum;
use serde::Serialize;

/// Available networks.
#[derive(Debug, Clone, ValueEnum, Serialize)]
pub enum Network {
    /// Optimism Mainnet
    Optimism,
    /// Optimism Sepolia
    OptimismSepolia,
}
