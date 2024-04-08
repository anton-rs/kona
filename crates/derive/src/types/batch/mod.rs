//! This module contains the batch types for the OP Stack derivation pipeline: [SpanBatch] &
//! [SingleBatch].

use super::DecodeError;
use crate::{
    traits::L2ChainProvider,
    types::{BlockInfo, L2BlockInfo, RollupConfig},
};
use alloc::vec::Vec;
use alloy_rlp::{Buf, Decodable, Encodable};

mod batch_type;
pub use batch_type::BatchType;

mod validity;
pub use validity::BatchValidity;

mod span_batch;
pub use span_batch::{
    RawSpanBatch, SpanBatch, SpanBatchBits, SpanBatchEip1559TransactionData,
    SpanBatchEip2930TransactionData, SpanBatchElement, SpanBatchError,
    SpanBatchLegacyTransactionData, SpanBatchPayload, SpanBatchPrefix, SpanBatchTransactionData,
    SpanBatchTransactions, SpanDecodingError, MAX_SPAN_BATCH_SIZE,
};

mod single_batch;
pub use single_batch::SingleBatch;

/// A batch with its inclusion block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchWithInclusionBlock {
    /// The inclusion block
    pub inclusion_block: BlockInfo,
    /// The batch
    pub batch: Batch,
}

impl BatchWithInclusionBlock {
    /// Validates the batch can be applied on top of the specified L2 safe head.
    /// The first entry of the l1_blocks should match the origin of the l2_safe_head.
    /// One or more consecutive l1_blocks should be provided.
    /// In case of only a single L1 block, the decision whether a batch is valid may have to stay
    /// undecided.
    pub fn check_batch<BF: L2ChainProvider>(
        &self,
        cfg: &RollupConfig,
        l1_blocks: &[BlockInfo],
        l2_safe_head: L2BlockInfo,
        fetcher: &BF,
    ) -> BatchValidity {
        match &self.batch {
            Batch::Single(single_batch) => {
                single_batch.check_batch(cfg, l1_blocks, l2_safe_head, &self.inclusion_block)
            }
            Batch::Span(span_batch) => {
                span_batch.check_batch(cfg, l1_blocks, l2_safe_head, &self.inclusion_block, fetcher)
            }
        }
    }
}

/// A Batch.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Batch {
    /// A single batch
    Single(SingleBatch),
    /// Span Batches
    Span(RawSpanBatch),
}

impl Batch {
    /// Returns the timestamp for the batch.
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::Single(sb) => sb.timestamp,
            Self::Span(sb) => sb.timestamp(),
        }
    }

    /// Attempts to encode a batch into a writer.
    pub fn encode(&self, w: &mut Vec<u8>) -> Result<(), DecodeError> {
        match self {
            Self::Single(single_batch) => {
                single_batch.encode(w);
                Ok(())
            }
            Self::Span(span_batch) => span_batch.encode(w).map_err(DecodeError::SpanBatchError),
        }
    }

    /// Attempts to decode a batch from a reader.
    pub fn decode(r: &mut &[u8]) -> Result<Self, DecodeError> {
        if r.is_empty() {
            return Err(DecodeError::EmptyBuffer);
        }

        // Read the batch type
        let batch_type = BatchType::from(r[0]);
        r.advance(1);

        match batch_type {
            BatchType::Single => {
                let single_batch = SingleBatch::decode(r)?;
                Ok(Batch::Single(single_batch))
            }
            BatchType::Span => {
                let span_batch = RawSpanBatch::decode(r).map_err(DecodeError::SpanBatchError)?;
                Ok(Batch::Span(span_batch))
            }
        }
    }
}
