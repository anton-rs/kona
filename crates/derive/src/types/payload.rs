//! Contains the execution payload type.

use alloc::vec::Vec;
use alloy_primitives::{Address, Bytes, B256, U256};

/// Fixed and variable memory costs for a payload.
/// ~1000 bytes per payload, with some margin for overhead like map data.
pub const PAYLOAD_MEM_FIXED_COST: u64 = 1000;

/// Memory overhead per payload transaction.
/// 24 bytes per tx overhead (size of slice header in memory).
pub const PAYLOAD_TX_MEM_OVERHEAD: u64 = 24;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Envelope wrapping the [ExecutionPayload].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPayloadEnvelope {
    /// Parent beacon block root.
    #[cfg_attr(feature = "serde", serde(rename = "parentBeaconBlockRoot"))]
    pub parent_beacon_block_root: Option<B256>,
    /// The inner execution payload.
    #[cfg_attr(feature = "serde", serde(rename = "executionPayload"))]
    pub execution_payload: ExecutionPayload,
}

impl ExecutionPayloadEnvelope {
    /// Returns the payload memory size.
    pub fn mem_size(&self) -> u64 {
        let mut out = PAYLOAD_MEM_FIXED_COST;
        for tx in &self.execution_payload.transactions {
            out += tx.len() as u64 + PAYLOAD_TX_MEM_OVERHEAD;
        }
        out
    }
}

/// The execution payload.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPayload {
    /// The parent hash.
    #[cfg_attr(feature = "serde", serde(rename = "parentHash"))]
    pub parent_hash: B256,
    /// The coinbase address.
    #[cfg_attr(feature = "serde", serde(rename = "feeRecipient"))]
    pub fee_recipient: Address,
    /// The state root.
    #[cfg_attr(feature = "serde", serde(rename = "stateRoot"))]
    pub state_root: B256,
    /// The transactions root.
    #[cfg_attr(feature = "serde", serde(rename = "receiptsRoot"))]
    pub receipts_root: B256,
    /// The logs bloom.
    #[cfg_attr(feature = "serde", serde(rename = "logsBloom"))]
    pub logs_bloom: B256,
    /// The mix hash.
    #[cfg_attr(feature = "serde", serde(rename = "prevRandao"))]
    pub prev_randao: B256,
    /// The difficulty.
    #[cfg_attr(feature = "serde", serde(rename = "blockNumber"))]
    pub block_number: u64,
    /// The gas limit.
    #[cfg_attr(feature = "serde", serde(rename = "gasLimit"))]
    pub gas_limit: u64,
    /// The gas used.
    #[cfg_attr(feature = "serde", serde(rename = "gasUsed"))]
    pub gas_used: u64,
    /// The timestamp.
    #[cfg_attr(feature = "serde", serde(rename = "timestamp"))]
    pub timestamp: u64,
    /// The extra data.
    #[cfg_attr(feature = "serde", serde(rename = "extraData"))]
    pub extra_data: B256,
    /// Base fee per gas.
    #[cfg_attr(feature = "serde", serde(rename = "baseFeePerGas"))]
    pub base_fee_per_gas: U256,
    /// Block hash.
    #[cfg_attr(feature = "serde", serde(rename = "blockHash"))]
    pub block_hash: B256,
    /// The transactions.
    #[cfg_attr(feature = "serde", serde(rename = "transactions"))]
    pub transactions: Vec<Bytes>,
    /// The withdrawals.
    #[cfg_attr(feature = "serde", serde(rename = "withdrawals"))]
    pub withdrawals: Option<Withdrawals>,
    /// The  blob gas used.
    #[cfg_attr(feature = "serde", serde(rename = "blobGasUsed"))]
    pub blob_gas_used: Option<u64>,
    /// The excess blob gas.
    #[cfg_attr(feature = "serde", serde(rename = "excessBlobGas"))]
    pub excess_blob_gas: Option<u64>,
}

/// Withdrawal Type
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withdrawals {}
