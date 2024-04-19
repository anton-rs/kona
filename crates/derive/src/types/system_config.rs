//! This module contains the [SystemConfig] type.

use super::RollupConfig;
use crate::{CONFIG_UPDATE_EVENT_VERSION_0, CONFIG_UPDATE_TOPIC};
use alloy_consensus::Receipt;
use alloy_primitives::{address, Address, Log, U256};
use alloy_sol_types::{sol, SolType};
use anyhow::{anyhow, bail, Result};

/// Optimism system config contract values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SystemConfig {
    /// Batch sender address
    pub batcher_addr: Address,
    /// L2 gas limit
    pub gas_limit: U256,
    /// Fee overhead
    #[cfg_attr(feature = "serde", serde(rename = "overhead"))]
    pub l1_fee_overhead: U256,
    /// Fee scalar
    #[cfg_attr(feature = "serde", serde(rename = "scalar"))]
    pub l1_fee_scalar: U256,
}

/// Represents type of update to the system config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SystemConfigUpdateType {
    /// Batcher update type
    Batcher = 0,
    /// Gas config update type
    GasConfig = 1,
    /// Gas limit update type
    GasLimit = 2,
    /// Unsafe block signer update type
    UnsafeBlockSigner = 3,
}

impl TryFrom<u64> for SystemConfigUpdateType {
    type Error = anyhow::Error;

    fn try_from(value: u64) -> core::prelude::v1::Result<Self, Self::Error> {
        match value {
            0 => Ok(SystemConfigUpdateType::Batcher),
            1 => Ok(SystemConfigUpdateType::GasConfig),
            2 => Ok(SystemConfigUpdateType::GasLimit),
            3 => Ok(SystemConfigUpdateType::UnsafeBlockSigner),
            _ => bail!("Invalid SystemConfigUpdateType value: {}", value),
        }
    }
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
            if !receipt.status {
                continue;
            }

            receipt.logs.iter().try_for_each(|log| {
                let topics = log.topics();
                if log.address == rollup_config.l1_system_config_address &&
                    !topics.is_empty() &&
                    topics[0] == CONFIG_UPDATE_TOPIC
                {
                    self.process_config_update_log(log, rollup_config, l1_time)?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    /// Decodes an EVM log entry emitted by the system config contract and applies it as a
    /// [SystemConfig] change.
    ///
    /// Parse log data for:
    ///
    /// ```text
    /// event ConfigUpdate(
    ///    uint256 indexed version,
    ///    UpdateType indexed updateType,
    ///    bytes data
    /// );
    /// ```
    fn process_config_update_log(
        &mut self,
        log: &Log,
        rollup_config: &RollupConfig,
        l1_time: u64,
    ) -> Result<()> {
        if log.topics().len() < 3 {
            bail!("Invalid config update log: not enough topics");
        }
        if log.topics()[0] != CONFIG_UPDATE_TOPIC {
            bail!("Invalid config update log: invalid topic");
        }

        // Parse the config update log
        let version = log.topics()[1];
        if version != CONFIG_UPDATE_EVENT_VERSION_0 {
            bail!("Invalid config update log: unsupported version");
        }
        let update_type = u64::from_be_bytes(
            log.topics()[2].as_slice()[24..]
                .try_into()
                .map_err(|_| anyhow!("Failed to convert update type to u64"))?,
        );
        let log_data = log.data.data.as_ref();

        match update_type.try_into()? {
            SystemConfigUpdateType::Batcher => {
                if log_data.len() != 96 {
                    bail!("Invalid config update log: invalid data length");
                }

                let pointer = <sol!(uint64)>::abi_decode(&log_data[0..32], true)
                    .map_err(|_| anyhow!("Failed to decode batcher update log"))?;
                if pointer != 32 {
                    bail!("Invalid config update log: invalid data pointer");
                }
                let length = <sol!(uint64)>::abi_decode(&log_data[32..64], true)
                    .map_err(|_| anyhow!("Failed to decode batcher update log"))?;
                if length != 32 {
                    bail!("Invalid config update log: invalid data length");
                }

                let batcher_address =
                    <sol!(address)>::abi_decode(&log.data.data.as_ref()[64..], true)
                        .map_err(|_| anyhow!("Failed to decode batcher update log"))?;
                self.batcher_addr = batcher_address;
            }
            SystemConfigUpdateType::GasConfig => {
                if log_data.len() != 128 {
                    bail!("Invalid config update log: invalid data length");
                }

                let pointer = <sol!(uint64)>::abi_decode(&log_data[0..32], true)
                    .map_err(|_| anyhow!("Invalid config update log: invalid data pointer"))?;
                if pointer != 32 {
                    bail!("Invalid config update log: invalid data pointer");
                }
                let length = <sol!(uint64)>::abi_decode(&log_data[32..64], true)
                    .map_err(|_| anyhow!("Invalid config update log: invalid data length"))?;
                if length != 64 {
                    bail!("Invalid config update log: invalid data length");
                }

                let overhead = <sol!(uint256)>::abi_decode(&log_data[64..96], true)
                    .map_err(|_| anyhow!("Invalid config update log: invalid overhead"))?;
                let scalar = <sol!(uint256)>::abi_decode(&log_data[96..], true)
                    .map_err(|_| anyhow!("Invalid config update log: invalid scalar"))?;

                if rollup_config.is_ecotone_active(l1_time) {
                    if RollupConfig::check_ecotone_l1_system_config_scalar(scalar.to_be_bytes())
                        .is_err()
                    {
                        // ignore invalid scalars, retain the old system-config scalar
                        return Ok(());
                    }

                    // retain the scalar data in encoded form
                    self.l1_fee_scalar = scalar;
                    // zero out the overhead, it will not affect the state-transition after Ecotone
                    self.l1_fee_overhead = U256::ZERO;
                } else {
                    self.l1_fee_scalar = scalar;
                    self.l1_fee_overhead = overhead;
                }
            }
            SystemConfigUpdateType::GasLimit => {
                if log_data.len() != 96 {
                    bail!("Invalid config update log: invalid data length");
                }

                let pointer = <sol!(uint64)>::abi_decode(&log_data[0..32], true)
                    .map_err(|_| anyhow!("Invalid config update log: invalid data pointer"))?;
                if pointer != 32 {
                    bail!("Invalid config update log: invalid data pointer");
                }
                let length = <sol!(uint64)>::abi_decode(&log_data[32..64], true)
                    .map_err(|_| anyhow!("Invalid config update log: invalid data length"))?;
                if length != 32 {
                    bail!("Invalid config update log: invalid data length");
                }

                let gas_limit = <sol!(uint256)>::abi_decode(&log_data[64..], true)
                    .map_err(|_| anyhow!("Invalid config update log: invalid gas limit"))?;
                self.gas_limit = gas_limit;
            }
            SystemConfigUpdateType::UnsafeBlockSigner => {
                // Ignored in derivation
            }
        }

        Ok(())
    }
}

/// System accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[cfg(test)]
mod test {
    use crate::types::Genesis;

    use super::*;
    use alloc::vec;
    use alloy_primitives::{b256, hex, LogData, B256};

    extern crate std;

    fn mock_rollup_config(system_config: SystemConfig) -> RollupConfig {
        RollupConfig {
            genesis: Genesis {
                l1: crate::types::BlockID::default(),
                l2: crate::types::BlockID::default(),
                timestamp: 0,
                system_config,
            },
            block_time: 2,
            max_sequencer_drift: 0,
            seq_window_size: 0,
            channel_timeout: 0,
            l1_chain_id: 1,
            l2_chain_id: 10,
            regolith_time: Some(0),
            canyon_time: Some(0),
            delta_time: Some(0),
            ecotone_time: Some(10),
            fjord_time: Some(0),
            interop_time: Some(0),
            batch_inbox_address: Address::ZERO,
            deposit_contract_address: Address::ZERO,
            l1_system_config_address: Address::ZERO,
            protocol_versions_address: Address::ZERO,
            blobs_enabled_l1_timestamp: Some(0),
            da_challenge_address: Some(Address::ZERO),
        }
    }

    #[test]
    fn test_system_config_update_batcher_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000000");

        let mut system_config = SystemConfig::default();
        let rollup_config = mock_rollup_config(system_config);

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the batcher address.
        system_config.process_config_update_log(&update_log, &rollup_config, 0).unwrap();

        assert_eq!(
            system_config.batcher_addr,
            address!("000000000000000000000000000000000000bEEF")
        );
    }

    #[test]
    fn test_system_config_update_gas_config_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000001");

        let mut system_config = SystemConfig::default();
        let rollup_config = mock_rollup_config(system_config);

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000babe000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the batcher address.
        system_config.process_config_update_log(&update_log, &rollup_config, 0).unwrap();

        assert_eq!(system_config.l1_fee_overhead, U256::from(0xbabe));
        assert_eq!(system_config.l1_fee_scalar, U256::from(0xbeef));
    }

    #[test]
    fn test_system_config_update_gas_config_log_ecotone() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000001");

        let mut system_config = SystemConfig::default();
        let rollup_config = mock_rollup_config(system_config);

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000babe000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the batcher address.
        system_config.process_config_update_log(&update_log, &rollup_config, 10).unwrap();

        assert_eq!(system_config.l1_fee_overhead, U256::from(0));
        assert_eq!(system_config.l1_fee_scalar, U256::from(0xbeef));
    }

    #[test]
    fn test_system_config_update_gas_limit_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000002");

        let mut system_config = SystemConfig::default();
        let rollup_config = mock_rollup_config(system_config);

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the batcher address.
        system_config.process_config_update_log(&update_log, &rollup_config, 0).unwrap();

        assert_eq!(system_config.gas_limit, U256::from(0xbeef));
    }
}
