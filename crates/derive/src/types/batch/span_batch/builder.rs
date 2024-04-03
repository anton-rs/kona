//! Span Batch Builder

#![allow(unused)]

use crate::types::{RawSpanBatch, SingleBatch, SpanBatch, SpanBatchElement};
use alloc::vec::Vec;
use alloy_primitives::FixedBytes;

/// The span batch builder builds a [SpanBatch] by adding
/// [SpanBatchElement] iteratively. Provides a way to stack
/// [SingleBatch]s and convert to [RawSpanBatch] for encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchBuilder {
    /// The genesis timestamp of the span
    genesis_timestamp: u64,
    /// The chain ID of the span
    chain_id: u64,
    /// The span batch
    span_batch: SpanBatch,
    /// The origin changed bit
    origin_changed_bit: u8,
}

impl SpanBatchBuilder {
    /// Create a new span batch builder
    pub fn new(genesis_timestamp: u64, chain_id: u64) -> Self {
        SpanBatchBuilder {
            genesis_timestamp,
            chain_id,
            span_batch: SpanBatch::default(),
            origin_changed_bit: 0,
        }
    }

    /// Gets the current lock count.
    pub fn get_block_count(&self) -> usize {
        self.span_batch.batches.len()
    }

    /// Resets the span batch builder.
    pub fn reset(&mut self) {
        self.span_batch = SpanBatch::default();
        self.origin_changed_bit = 0;
    }

    /// Returns the raw span batch ready for encoding.
    pub fn get_raw_span_batch(&self) -> RawSpanBatch {
        // self.span_batch.to_raw_span_batch(
        //     self.origin_changed_bit,
        //     self.genesis_timestamp,
        //     self.chain_id,
        // )
        unimplemented!()
    }

    /// Append a singular batch to the span batch and update the origin changed bit
    pub fn append_singular_batch(&mut self, _singular_batch: &SingleBatch, _seq_num: u64) {
        // if self.get_block_count() == 0 {
        //     self.origin_changed_bit = 0;
        //     if seq_num == 0 {
        //         self.origin_changed_bit = 1;
        //     }
        // }
        // self.span_batch.batches.push(SpanBatchElement {
        //     epoch_num: singular_batch.epoch_num,
        //     timestamp: singular_batch.timestamp,
        //     transactions: singular_batch.transactions.clone(),
        // });
        unimplemented!()
    }
}
