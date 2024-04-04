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
    parent_beacon_block_root: Option<B256>,
    /// The inner execution payload.
    #[cfg_attr(feature = "serde", serde(rename = "executionPayload"))]
    execution_payload: ExecutionPayload,
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
    #[cfg_attr(feature = "serde", serde(rename = "parentHash"))]
    parent_hash: B256,
    #[cfg_attr(feature = "serde", serde(rename = "feeRecipient"))]
    fee_recipient: Address,
    #[cfg_attr(feature = "serde", serde(rename = "stateRoot"))]
    state_root: B256,
    #[cfg_attr(feature = "serde", serde(rename = "receiptsRoot"))]
    receipts_root: B256,
    #[cfg_attr(feature = "serde", serde(rename = "logsBloom"))]
    logs_bloom: B256,
    #[cfg_attr(feature = "serde", serde(rename = "prevRandao"))]
    prev_randao: B256,
    #[cfg_attr(feature = "serde", serde(rename = "blockNumber"))]
    block_number: u64,
    #[cfg_attr(feature = "serde", serde(rename = "gasLimit"))]
    gas_limit: u64,
    #[cfg_attr(feature = "serde", serde(rename = "gasUsed"))]
    gas_used: u64,
    #[cfg_attr(feature = "serde", serde(rename = "timestamp"))]
    timestamp: u64,
    #[cfg_attr(feature = "serde", serde(rename = "extraData"))]
    extra_data: B256,
    #[cfg_attr(feature = "serde", serde(rename = "baseFeePerGas"))]
    base_fee_per_gas: U256,
    #[cfg_attr(feature = "serde", serde(rename = "blockHash"))]
    block_hash: B256,
    #[cfg_attr(feature = "serde", serde(rename = "transactions"))]
    transactions: Vec<Bytes>,
    #[cfg_attr(feature = "serde", serde(rename = "withdrawals"))]
    withdrawals: Option<Withdrawals>,
    #[cfg_attr(feature = "serde", serde(rename = "blobGasUsed"))]
    blob_gas_used: Option<u64>,
    #[cfg_attr(feature = "serde", serde(rename = "excessBlobGas"))]
    excess_blob_gas: Option<u64>,
}

/// Withdrawal Type
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withdrawals {}
