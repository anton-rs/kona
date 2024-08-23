//! This module contains the parameters and identifying types for the derivation pipeline.

use alloy_primitives::{address, b256, Address, B256};

/// The sequencer fee vault address.
pub const SEQUENCER_FEE_VAULT_ADDRESS: Address =
    address!("4200000000000000000000000000000000000011");

/// `keccak256("ConfigUpdate(uint256,uint8,bytes)")`
pub const CONFIG_UPDATE_TOPIC: B256 =
    b256!("1d2b0bda21d56b8bd12d4f94ebacffdfb35f5e226f84b461103bb8beab6353be");

/// The initial version of the system config event log.
pub const CONFIG_UPDATE_EVENT_VERSION_0: B256 = B256::ZERO;

/// The maximum size of a channel bank.
pub const MAX_CHANNEL_BANK_SIZE: usize = 100_000_000;

/// The maximum size of a channel bank after the Fjord Hardfork.
pub const FJORD_MAX_CHANNEL_BANK_SIZE: usize = 1_000_000_000;
