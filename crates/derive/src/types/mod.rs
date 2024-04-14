//! This module contains all of the types used within the derivation pipeline.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use alloc::vec::Vec;
pub use alloy_consensus::Receipt;
use alloy_primitives::Bytes;
use alloy_rlp::{Decodable, Encodable};

mod attributes;
pub use attributes::{L2AttributesWithParent, L2PayloadAttributes};

mod system_config;
pub use system_config::{SystemAccounts, SystemConfig, SystemConfigUpdateType};

mod deposits;
pub use deposits::*;

mod rollup_config;
pub use rollup_config::RollupConfig;

pub mod batch;
pub use batch::{
    Batch, BatchType, BatchValidity, BatchWithInclusionBlock, RawSpanBatch, SingleBatch, SpanBatch,
    SpanBatchBits, SpanBatchEip1559TransactionData, SpanBatchEip2930TransactionData,
    SpanBatchElement, SpanBatchError, SpanBatchLegacyTransactionData, SpanBatchPayload,
    SpanBatchPrefix, SpanBatchTransactionData, SpanBatchTransactions, SpanDecodingError,
    MAX_SPAN_BATCH_SIZE,
};

mod ecotone;
pub use ecotone::*;

mod payload;
pub use payload::{
    L2ExecutionPayload, L2ExecutionPayloadEnvelope, PAYLOAD_MEM_FIXED_COST, PAYLOAD_TX_MEM_OVERHEAD,
};

mod block;
pub use block::{Block, BlockID, BlockInfo, BlockKind, L2BlockInfo, OpBlock, Withdrawal};

mod l1_block_info;
pub use l1_block_info::{L1BlockInfoBedrock, L1BlockInfoEcotone, L1BlockInfoTx};

mod blob;
pub use blob::{Blob, BlobData, BlobDecodingError, IndexedBlobHash};

mod genesis;
pub use genesis::Genesis;

mod frame;
pub use frame::Frame;

mod channel;
pub use channel::Channel;

mod errors;
pub use errors::*;

/// A raw transaction
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RawTransaction(pub Bytes);

impl RawTransaction {
    /// Returns if the transaction is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns if the transaction is a deposit
    pub fn is_deposit(&self) -> bool {
        !self.0.is_empty() && self.0[0] == 0x7E
    }
}

impl<T: Into<Bytes>> From<T> for RawTransaction {
    fn from(bytes: T) -> Self {
        Self(bytes.into())
    }
}

impl Encodable for RawTransaction {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.0.encode(out)
    }
}

impl Decodable for RawTransaction {
    /// Decodes RLP encoded bytes into [RawTransaction] bytes
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let tx_bytes = Bytes::decode(buf)?;
        Ok(Self(tx_bytes))
    }
}

impl AsRef<[u8]> for RawTransaction {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// A single L2 block derived from a batch.
#[derive(Debug, Clone)]
pub struct BlockInput {
    /// Timestamp of the L2 block
    pub timestamp: u64,
    /// Transactions included in this block
    pub transactions: Vec<RawTransaction>,
    /// The L1 block this batch was fully derived from
    pub l1_inclusion_block: u64,
}
