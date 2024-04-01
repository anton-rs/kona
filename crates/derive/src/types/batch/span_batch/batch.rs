//! The Span Batch Type

#![allow(unused)]

use crate::types::{
    RawSpanBatch, SpanBatchBits, SpanBatchElement, SpanBatchPayload, SpanBatchPrefix,
};
use alloc::{vec, vec::Vec};
use alloy_primitives::FixedBytes;

use super::SpanBatchError;

/// The span batch contains the input to build a span of L2 blocks in derived form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatch {
    /// First 20 bytes of the first block's parent hash
    pub parent_check: FixedBytes<20>,
    /// First 20 bytes of the last block's L1 origin hash
    pub l1_origin_check: FixedBytes<20>,
    /// List of block input in derived form
    pub batches: Vec<SpanBatchElement>,
}

impl SpanBatch {
    /// Returns the timestamp for the first batch in the span.
    pub fn get_timestamp(&self) -> u64 {
        self.batches[0].timestamp
    }

    /// Converts the span batch to a raw span batch.
    pub fn to_raw_span_batch(
        &self,
        origin_changed_bit: u8,
        genesis_timestamp: u64,
        chain_id: u64,
    ) -> Result<RawSpanBatch, SpanBatchError> {
        if self.batches.is_empty() {
            return Err(SpanBatchError::EmptySpanBatch);
        }

        let span_start = self.batches.first().ok_or(SpanBatchError::EmptySpanBatch)?;
        let span_end = self.batches.last().ok_or(SpanBatchError::EmptySpanBatch)?;

        // TODO: Need to expand the [SpanBatch] type, as implemented in `op-node`. It should have extra data, incl.
        // the origin bits, block tx counts, and span batch txs.
        Ok(RawSpanBatch {
            prefix: SpanBatchPrefix {
                rel_timestamp: span_start.timestamp - genesis_timestamp,
                l1_origin_num: span_end.epoch_num,
                parent_check: self.parent_check,
                l1_origin_check: self.l1_origin_check,
            },
            payload: SpanBatchPayload {
                block_count: self.batches.len() as u64,
                origin_bits: todo!(),
                block_tx_counts: todo!(),
                txs: todo!(),
            },
        })
    }
}
