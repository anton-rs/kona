//! This module contains the [RollupConfig] type.

use crate::genesis::Genesis;
use alloy_eips::eip1559::BaseFeeParams;
use alloy_primitives::Address;

/// Base fee max change denominator for Optimism Mainnet as defined in the Optimism
/// [transaction costs](https://community.optimism.io/docs/developers/build/differences/#transaction-costs) doc.
pub const OP_MAINNET_EIP1559_DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR: u128 = 50;

/// Base fee max change denominator for Optimism Mainnet as defined in the Optimism Canyon
/// hardfork.
pub const OP_MAINNET_EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR_CANYON: u128 = 250;

/// Base fee max change denominator for Optimism Mainnet as defined in the Optimism
/// [transaction costs](https://community.optimism.io/docs/developers/build/differences/#transaction-costs) doc.
pub const OP_MAINNET_EIP1559_DEFAULT_ELASTICITY_MULTIPLIER: u128 = 6;

/// Base fee max change denominator for Optimism Sepolia as defined in the Optimism
/// [transaction costs](https://community.optimism.io/docs/developers/build/differences/#transaction-costs) doc.
pub const OP_SEPOLIA_EIP1559_DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR: u128 = 50;

/// Base fee max change denominator for Optimism Sepolia as defined in the Optimism Canyon
/// hardfork.
pub const OP_SEPOLIA_EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR_CANYON: u128 = 250;

/// Base fee max change denominator for Optimism Sepolia as defined in the Optimism
/// [transaction costs](https://community.optimism.io/docs/developers/build/differences/#transaction-costs) doc.
pub const OP_SEPOLIA_EIP1559_DEFAULT_ELASTICITY_MULTIPLIER: u128 = 10;

/// Get the base fee parameters for Optimism Sepolia.
pub const OP_SEPOLIA_BASE_FEE_PARAMS: BaseFeeParams = BaseFeeParams {
    max_change_denominator: OP_SEPOLIA_EIP1559_DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR,
    elasticity_multiplier: OP_SEPOLIA_EIP1559_DEFAULT_ELASTICITY_MULTIPLIER,
};

/// Get the base fee parameters for Optimism Sepolia (post Canyon).
pub const OP_SEPOLIA_CANYON_BASE_FEE_PARAMS: BaseFeeParams = BaseFeeParams {
    max_change_denominator: OP_SEPOLIA_EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR_CANYON,
    elasticity_multiplier: OP_SEPOLIA_EIP1559_DEFAULT_ELASTICITY_MULTIPLIER,
};

/// Get the base fee parameters for Optimism Mainnet.
pub const OP_BASE_FEE_PARAMS: BaseFeeParams = BaseFeeParams {
    max_change_denominator: OP_MAINNET_EIP1559_DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR,
    elasticity_multiplier: OP_MAINNET_EIP1559_DEFAULT_ELASTICITY_MULTIPLIER,
};

/// Get the base fee parameters for Optimism Mainnet (post Canyon).
pub const OP_CANYON_BASE_FEE_PARAMS: BaseFeeParams = BaseFeeParams {
    max_change_denominator: OP_MAINNET_EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR_CANYON,
    elasticity_multiplier: OP_MAINNET_EIP1559_DEFAULT_ELASTICITY_MULTIPLIER,
};

/// The Rollup configuration.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RollupConfig {
    /// The genesis state of the rollup.
    pub genesis: Genesis,
    /// The block time of the L2, in seconds.
    pub block_time: u64,
    /// Sequencer batches may not be more than MaxSequencerDrift seconds after
    /// the L1 timestamp of the sequencing window end.
    ///
    /// Note: When L1 has many 1 second consecutive blocks, and L2 grows at fixed 2 seconds,
    /// the L2 time may still grow beyond this difference.
    pub max_sequencer_drift: u64,
    /// The sequencer window size.
    pub seq_window_size: u64,
    /// Number of L1 blocks between when a channel can be opened and when it can be closed.
    pub channel_timeout: u64,
    /// The L1 chain ID
    pub l1_chain_id: u64,
    /// The L2 chain ID
    pub l2_chain_id: u64,
    /// `regolith_time` sets the activation time of the Regolith network-upgrade:
    /// a pre-mainnet Bedrock change that addresses findings of the Sherlock contest related to
    /// deposit attributes. "Regolith" is the loose deposited rock that sits on top of Bedrock.
    /// Active if regolith_time != None && L2 block timestamp >= Some(regolith_time), inactive
    /// otherwise.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub regolith_time: Option<u64>,
    /// `canyon_time` sets the activation time of the Canyon network upgrade.
    /// Active if `canyon_time` != None && L2 block timestamp >= Some(canyon_time), inactive
    /// otherwise.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub canyon_time: Option<u64>,
    /// `delta_time` sets the activation time of the Delta network upgrade.
    /// Active if `delta_time` != None && L2 block timestamp >= Some(delta_time), inactive
    /// otherwise.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub delta_time: Option<u64>,
    /// `ecotone_time` sets the activation time of the Ecotone network upgrade.
    /// Active if `ecotone_time` != None && L2 block timestamp >= Some(ecotone_time), inactive
    /// otherwise.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ecotone_time: Option<u64>,
    /// `fjord_time` sets the activation time of the Fjord network upgrade.
    /// Active if `fjord_time` != None && L2 block timestamp >= Some(fjord_time), inactive
    /// otherwise.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub fjord_time: Option<u64>,
    /// `interop_time` sets the activation time for an experimental feature-set, activated like a
    /// hardfork. Active if `interop_time` != None && L2 block timestamp >= Some(interop_time),
    /// inactive otherwise.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub interop_time: Option<u64>,
    /// `batch_inbox_address` is the L1 address that batches are sent to.
    pub batch_inbox_address: Address,
    /// `deposit_contract_address` is the L1 address that deposits are sent to.
    pub deposit_contract_address: Address,
    /// `l1_system_config_address` is the L1 address that the system config is stored at.
    pub l1_system_config_address: Address,
    /// `protocol_versions_address` is the L1 address that the protocol versions are stored at.
    pub protocol_versions_address: Address,
    /// `blobs_enabled_l1_timestamp` is the timestamp to start reading blobs as a batch data
    /// source. Optional.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "blobs_data", skip_serializing_if = "Option::is_none")
    )]
    pub blobs_enabled_l1_timestamp: Option<u64>,
    /// `da_challenge_address` is the L1 address that the data availability challenge contract is
    /// stored at.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub da_challenge_address: Option<Address>,
}

impl RollupConfig {
    /// Returns true if Regolith is active at the given timestamp.
    pub fn is_regolith_active(&self, timestamp: u64) -> bool {
        self.regolith_time.map_or(false, |t| timestamp >= t)
    }

    /// Returns true if Canyon is active at the given timestamp.
    pub fn is_canyon_active(&self, timestamp: u64) -> bool {
        self.canyon_time.map_or(false, |t| timestamp >= t)
    }

    /// Returns true if Delta is active at the given timestamp.
    pub fn is_delta_active(&self, timestamp: u64) -> bool {
        self.delta_time.map_or(false, |t| timestamp >= t)
    }

    /// Returns true if Ecotone is active at the given timestamp.
    pub fn is_ecotone_active(&self, timestamp: u64) -> bool {
        self.ecotone_time.map_or(false, |t| timestamp >= t)
    }

    /// Returns true if Fjord is active at the given timestamp.
    pub fn is_fjord_active(&self, timestamp: u64) -> bool {
        self.fjord_time.map_or(false, |t| timestamp >= t)
    }

    /// Returns true if Interop is active at the given timestamp.
    pub fn is_interop_active(&self, timestamp: u64) -> bool {
        self.interop_time.map_or(false, |t| timestamp >= t)
    }

    /// Returns true if a DA Challenge proxy Address is provided in the rollup config and the
    /// address is not zero.
    pub fn is_plasma_enabled(&self) -> bool {
        self.da_challenge_address.map_or(false, |addr| !addr.is_zero())
    }

    /// Returns the base fee parameters for the given rollup configuration.
    ///
    /// TODO: Consider chain-specific base fee params (i.e. Sepolia vs. Mainnet).
    pub fn base_fee_params_at_timestamp(&self, timestamp: u64) -> BaseFeeParams {
        if self.is_canyon_active(timestamp) {
            OP_CANYON_BASE_FEE_PARAMS
        } else {
            OP_BASE_FEE_PARAMS
        }
    }

    /// Checks the scalar value in Ecotone.
    pub fn check_ecotone_l1_system_config_scalar(scalar: [u8; 32]) -> Result<(), &'static str> {
        let version_byte = scalar[0];
        match version_byte {
            0 => {
                if scalar[1..28] != [0; 27] {
                    return Err("Bedrock scalar padding not empty");
                }
                Ok(())
            }
            1 => {
                if scalar[1..24] != [0; 23] {
                    return Err("Invalid version 1 scalar padding");
                }
                Ok(())
            }
            _ => {
                // ignore the event if it's an unknown scalar format
                Err("Unrecognized scalar version")
            }
        }
    }
}
