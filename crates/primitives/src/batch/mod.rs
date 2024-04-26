//! This module contains the batch types for the OP Stack derivation pipeline: [SpanBatch] &
//! [SingleBatch].

use crate::block::{BlockInfo, L2BlockInfo};
use crate::rollup_config::RollupConfig;

use alloy_rlp::{Buf, Decodable};

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

use core::fmt::Display;
use alloc::sync::Arc;
use crate::payload::L2ExecutionPayloadEnvelope;
use crate::system_config::SystemConfig;
use async_trait::async_trait;
use alloc::boxed::Box;

// TODO: remove and split up the span batch check so it doesn't need an L2ChainProvider argument
/// Describes the functionality of a data source that fetches safe blocks.
#[async_trait]
pub trait L2ChainProvider {
    /// Returns the L2 block info given a block number.
    /// Errors if the block does not exist.
    async fn l2_block_info_by_number(&mut self, number: u64) -> anyhow::Result<L2BlockInfo>;

    /// Returns an execution payload for a given number.
    /// Errors if the execution payload does not exist.
    async fn payload_by_number(&mut self, number: u64) -> anyhow::Result<L2ExecutionPayloadEnvelope>;

    /// Returns the [SystemConfig] by L2 number.
    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> anyhow::Result<SystemConfig>;
}

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
    pub async fn check_batch<BF: L2ChainProvider>(
        &self,
        cfg: &RollupConfig,
        l1_blocks: &[BlockInfo],
        l2_safe_head: L2BlockInfo,
        fetcher: &mut BF,
    ) -> BatchValidity {
        match &self.batch {
            Batch::Single(single_batch) => {
                single_batch.check_batch(cfg, l1_blocks, l2_safe_head, &self.inclusion_block)
            }
            Batch::Span(span_batch) => {
                span_batch
                    .check_batch(cfg, l1_blocks, l2_safe_head, &self.inclusion_block, fetcher)
                    .await
            }
        }
    }
}

/// A Batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Batch {
    /// A single batch
    Single(SingleBatch),
    /// Span Batches
    Span(SpanBatch),
}

impl Batch {
    /// Returns the timestamp for the batch.
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::Single(sb) => sb.timestamp,
            Self::Span(sb) => sb.timestamp(),
        }
    }

    /// Attempts to decode a batch from a reader.
    pub fn decode(r: &mut &[u8], cfg: &RollupConfig) -> Result<Self, DecodeError> {
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
                let mut raw_span_batch =
                    RawSpanBatch::decode(r).map_err(DecodeError::SpanBatchError)?;
                let span_batch = raw_span_batch
                    .derive(cfg.block_time, cfg.genesis.timestamp, cfg.l2_chain_id)
                    .map_err(DecodeError::SpanBatchError)?;
                Ok(Batch::Span(span_batch))
            }
        }
    }
}

/// A decoding error.
#[derive(Debug)]
pub enum DecodeError {
    /// The buffer is empty.
    EmptyBuffer,
    /// Alloy RLP Encoding Error.
    AlloyRlpError(alloy_rlp::Error),
    /// Span Batch Error.
    SpanBatchError(SpanBatchError),
}

impl From<alloy_rlp::Error> for DecodeError {
    fn from(e: alloy_rlp::Error) -> Self {
        DecodeError::AlloyRlpError(e)
    }
}

impl PartialEq<DecodeError> for DecodeError {
    fn eq(&self, other: &DecodeError) -> bool {
        matches!(
            (self, other),
            (DecodeError::EmptyBuffer, DecodeError::EmptyBuffer) |
                (DecodeError::AlloyRlpError(_), DecodeError::AlloyRlpError(_))
        )
    }
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::EmptyBuffer => write!(f, "Empty buffer"),
            DecodeError::AlloyRlpError(e) => write!(f, "Alloy RLP Decoding Error: {}", e),
            DecodeError::SpanBatchError(e) => write!(f, "Span Batch Decoding Error: {:?}", e),
        }
    }
}
