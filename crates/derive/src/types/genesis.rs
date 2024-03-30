//! This module contains the [Genesis] type.

use super::{BlockId, SystemConfig};

/// Represents the genesis state of the rollup.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Genesis {
    /// The L1 block that the rollup starts *after* (no derived transactions)
    pub l1: BlockId,
    /// The L2 block the rollup starts from (no transactions, pre-configured state)
    pub l2: BlockId,
    /// Timestamp of the L2 block.
    pub timestamp: u64,
    /// Initial system configuration values.
    /// The L2 genesis block may not include transactions, and thus cannot encode the config values,
    /// unlike later L2 blocks.
    pub system_config: SystemConfig,
}
