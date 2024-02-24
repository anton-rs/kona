//! This module contains the [SystemConfig] type.

use super::{Receipt, RollupConfig};
use alloy_primitives::{address, b256, Address, Log, B256, U256};
use anyhow::Result;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// `keccak256("ConfigUpdate(uint256,uint8,bytes)")`
const CONFIG_UPDATE_TOPIC: B256 =
    b256!("1d2b0bda21d56b8bd12d4f94ebacffdfb35f5e226f84b461103bb8beab6353be");

/// Optimism system config contract values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SystemConfig {
    /// Batch sender address
    pub batch_sender: Address,
    /// L2 gas limit
    pub gas_limit: U256,
    /// Fee overhead
    pub l1_fee_overhead: U256,
    /// Fee scalar
    pub l1_fee_scalar: U256,
    /// Sequencer's signer for unsafe blocks
    pub unsafe_block_signer: Address,
}

impl SystemConfig {
    /// Filters all L1 receipts to find config updates and applies the config updates.
    pub fn update_with_receipts(
        &mut self,
        receipts: &[Receipt],
        rollup_config: &RollupConfig,
        l1_time: u64,
    ) -> Result<()> {
        for receipt in receipts {
            if !receipt.success {
                continue;
            }

            for log in receipt.logs.iter() {
                let topics = log.topics();
                // TODO: System config address isn't in this type, replace `Address::ZERO`.
                if log.address == Address::ZERO
                    && !topics.is_empty()
                    && topics[0] == CONFIG_UPDATE_TOPIC
                {
                    self.process_config_update_log(log, rollup_config, l1_time)?;
                }
            }
        }
        Ok(())
    }

    /// Processes a single config update log.
    fn process_config_update_log(&mut self, _: &Log, _: &RollupConfig, _: u64) -> Result<()> {
        todo!("Process log update event.");
    }
}

/// System accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SystemAccounts {
    /// The address that can deposit attributes
    pub attributes_depositor: Address,
    /// The address of the attributes predeploy
    pub attributes_predeploy: Address,
    /// The address of the fee vault
    pub fee_vault: Address,
}

impl Default for SystemAccounts {
    fn default() -> Self {
        Self {
            attributes_depositor: address!("deaddeaddeaddeaddeaddeaddeaddeaddead0001"),
            attributes_predeploy: address!("4200000000000000000000000000000000000015"),
            fee_vault: address!("4200000000000000000000000000000000000011"),
        }
    }
}
