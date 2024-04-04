//! This module contains all of the types used within the derivation pipeline.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use alloc::vec::Vec;
use alloy_primitives::Bytes;
use alloy_rlp::{Decodable, Encodable};

mod attributes;
pub use attributes::{AttributesWithParent, PayloadAttributes};

mod system_config;
pub use system_config::{SystemAccounts, SystemConfig, SystemConfigUpdateType};

mod rollup_config;
pub use rollup_config::RollupConfig;

pub mod batch;
pub use batch::{
    Batch, BatchType, BatchValidity, BatchWithInclusionBlock, RawSpanBatch, SingleBatch, SpanBatch,
    SpanBatchBits, SpanBatchBuilder, SpanBatchEip1559TransactionData,
    SpanBatchEip2930TransactionData, SpanBatchElement, SpanBatchError,
    SpanBatchLegacyTransactionData, SpanBatchPayload, SpanBatchPrefix, SpanBatchTransactionData,
    SpanBatchTransactions, SpanDecodingError, MAX_SPAN_BATCH_SIZE,
};

mod alloy;
pub use alloy::{
    calc_blob_gasprice, calc_excess_blob_gas, calc_next_block_base_fee, eip1559, eip2718, eip2930,
    eip4788, eip4844, Header, NetworkReceipt, Receipt, ReceiptWithBloom, Sealable, Sealed, Signed,
    Transaction, TxDeposit, TxEip1559, TxEip2930, TxEip4844, TxEnvelope, TxKind, TxLegacy, TxType,
    EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH,
};

mod payload;
pub use payload::{
    ExecutionPayload, ExecutionPayloadEnvelope, PAYLOAD_MEM_FIXED_COST, PAYLOAD_TX_MEM_OVERHEAD,
};

mod block;
pub use block::{BlockID, BlockInfo, BlockKind, L2BlockInfo};

mod blob;
pub use blob::{Blob, BlobData, IndexedBlobHash};

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
