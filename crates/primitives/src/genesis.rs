//! This module contains the [Genesis] type.

use crate::{block::BlockID, system_config::SystemConfig};
use alloy_primitives::{address, b256, U256};

/// Represents the genesis state of the rollup.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Genesis {
    /// The L1 block that the rollup starts *after* (no derived transactions)
    pub l1: BlockID,
    /// The L2 block the rollup starts from (no transactions, pre-configured state)
    pub l2: BlockID,
    /// Timestamp of the L2 block.
    pub timestamp: u64,
    /// Initial system configuration values.
    /// The L2 genesis block may not include transactions, and thus cannot encode the config
    /// values, unlike later L2 blocks.
    pub system_config: SystemConfig,
}

impl Genesis {
    /// Returns the genesis configuration for OP Mainnet.
    pub fn op_mainnet() -> Self {
        Self {
            l1: BlockID {
                hash: b256!("438335a20d98863a4c0c97999eb2481921ccd28553eac6f913af7c12aec04108"),
                number: 17_422_590_u64,
            },
            l2: BlockID {
                hash: b256!("dbf6a80fef073de06add9b0d14026d6e5a86c85f6d102c36d3d8e9cf89c2afd3"),
                number: 105_235_063_u64,
            },
            timestamp: 1_686_068_903_u64,
            system_config: SystemConfig {
                batcher_addr: address!("6887246668a3b87f54deb3b94ba47a6f63f32985"),
                l1_fee_overhead: U256::from(0xbc),
                l1_fee_scalar: U256::from(0xa6fe0),
                gas_limit: U256::from(30_000_000_u64),
            },
        }
    }
}
