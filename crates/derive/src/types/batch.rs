//! This module contains the enumerable [Batch].

use super::batch_type::BatchType;
use super::batch_validity::BatchValidity;
use super::block::BlockInfo;
use super::block::L2BlockRef;
use super::rollup_config::RollupConfig;
use super::single_batch::SingleBatch;
use crate::traits::SafeBlockFetcher;

use alloy_rlp::{Decodable, Encodable};

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
    /// In case of only a single L1 block, the decision whether a batch is valid may have to stay undecided.
    pub fn check_batch<BF: SafeBlockFetcher>(
        &self,
        cfg: &RollupConfig,
        l1_blocks: &[BlockInfo],
        l2_safe_head: L2BlockRef,
        fetcher: &BF,
    ) -> BatchValidity {
        match &self.batch {
            Batch::Single(single_batch) => single_batch.check_batch(
                cfg,
                l1_blocks,
                l2_safe_head,
                &self.inclusion_block,
                fetcher,
            ),
            Batch::Span(span_batch) => {
                span_batch.check_batch(cfg, l1_blocks, l2_safe_head, &self.inclusion_block, fetcher)
            }
        }
    }
}

/// Span Batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatch {}

impl SpanBatch {
    /// Returns the timestamp for the span batch.
    pub fn timestamp(&self) -> u64 {
        unimplemented!()
    }

    /// Checks if the batch is valid.
    pub fn check_batch<BF: SafeBlockFetcher>(
        &self,
        _cfg: &RollupConfig,
        _l1_blocks: &[BlockInfo],
        _l2_safe_head: L2BlockRef,
        _inclusion_block: &BlockInfo,
        _fetcher: &BF,
    ) -> BatchValidity {
        unimplemented!()
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
}

impl Decodable for Batch {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        // The buffer must have at least one identifier byte.
        if buf.is_empty() {
            return Err(alloy_rlp::Error::Custom("Empty buffer"));
        }
        match BatchType::from(buf[0]) {
            BatchType::Single => {
                let single_batch = SingleBatch::decode(buf)?;
                Ok(Batch::Single(single_batch))
            }
            BatchType::Span => {
                // TODO: implement span batch decoding
                unimplemented!()
            }
        }
    }
}

impl Encodable for Batch {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Batch::Single(single_batch) => {
                BatchType::Single.encode(out);
                single_batch.encode(out);
            }
            Batch::Span(_) => {
                // TODO: implement span batch encoding
                unimplemented!()
            }
        }
    }
}
