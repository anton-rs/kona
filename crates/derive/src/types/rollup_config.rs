//! This module contains the [RollupConfig] type.

use alloy_primitives::Address;

/// The Rollup configuration.
#[derive(Debug, Clone, Copy)]
pub struct RollupConfig {
    /// The block time of the L2, in seconds.
    pub block_time: u64,
    /// Sequencer batches may not be more than MaxSequencerDrift seconds after
    /// the L1 timestamp of the sequencing window end.
    ///
    /// Note: When L1 has many 1 second consecutive blocks, and L2 grows at fixed 2 seconds,
    /// the L2 time may still grow beyond this difference.
    pub max_sequencer_drift: u64,
    /// The sequencer window size.
    pub sequencer_window_size: u64,
    /// Number of L1 blocks between when a channel can be opened and when it can be closed.
    pub channel_timeout: u64,
    /// The L1 chain ID
    pub l1_chain_id: u64,
    /// The L2 chain ID
    pub l2_chain_id: u64,
    /// `regolith_time` sets the activation time of the Regolith network-upgrade:
    /// a pre-mainnet Bedrock change that addresses findings of the Sherlock contest related to deposit attributes.
    /// "Regolith" is the loose deposited rock that sits on top of Bedrock.
    /// Active if regolith_time != None && L2 block timestamp >= Some(regolith_time), inactive otherwise.
    pub regolith_time: Option<u64>,
    /// `canyon_time` sets the activation time of the Canyon network upgrade.
    /// Active if `canyon_time` != None && L2 block timestamp >= Some(canyon_time), inactive otherwise.
    pub canyon_time: Option<u64>,
    /// `delta_time` sets the activation time of the Delta network upgrade.
    /// Active if `delta_time` != None && L2 block timestamp >= Some(delta_time), inactive otherwise.
    pub delta_time: Option<u64>,
    /// `ecotone_time` sets the activation time of the Ecotone network upgrade.
    /// Active if `ecotone_time` != None && L2 block timestamp >= Some(ecotone_time), inactive otherwise.
    pub ecotone_time: Option<u64>,
    /// `fjord_time` sets the activation time of the Fjord network upgrade.
    /// Active if `fjord_time` != None && L2 block timestamp >= Some(fjord_time), inactive otherwise.
    pub fjord_time: Option<u64>,
    /// `interop_time` sets the activation time for an experimental feature-set, activated like a hardfork.
    /// Active if `interop_time` != None && L2 block timestamp >= Some(interop_time), inactive otherwise.
    pub interop_time: Option<u64>,
    /// `batch_inbox_address` is the L1 address that batches are sent to.
    pub batch_inbox_address: Address,
    /// `deposit_contract_address` is the L1 address that deposits are sent to.
    pub deposit_contract_address: Address,
    /// `l1_system_config_address` is the L1 address that the system config is stored at.
    pub l1_system_config_address: Address,
    /// `protocol_versions_address` is the L1 address that the protocol versions are stored at.
    pub protocol_versions_address: Address,
    /// `blobs_enabled_l1_timestamp` is the timestamp to start reading blobs as a batch data source. Optional.
    pub blobs_enabled_l1_timestamp: Option<u64>,
    /// `da_challenge_address` is the L1 address that the data availability challenge contract is stored at.
    pub da_challenge_address: Option<Address>,
}
