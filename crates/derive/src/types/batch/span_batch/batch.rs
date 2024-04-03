//! The Span Batch Type

#![allow(unused)]

use super::{SpanBatchError, SpanBatchTransactions};
use crate::{
    traits::SafeBlockFetcher,
    types::{
        block::L2BlockInfo, BatchValidity, BlockInfo, L2BlockRef, RawSpanBatch, RollupConfig,
        SingleBatch, SpanBatchBits, SpanBatchElement, SpanBatchPayload, SpanBatchPrefix,
    },
};
use alloc::{vec, vec::Vec};
use alloy_primitives::FixedBytes;

/// The span batch contains the input to build a span of L2 blocks in derived form.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SpanBatch {
    /// First 20 bytes of the first block's parent hash
    pub parent_check: FixedBytes<20>,
    /// First 20 bytes of the last block's L1 origin hash
    pub l1_origin_check: FixedBytes<20>,
    /// Genesis block timestamp
    pub genesis_timestamp: u64,
    /// Chain ID
    pub chain_id: u64,
    /// List of block input in derived form
    pub batches: Vec<SpanBatchElement>,
    /// Caching - origin bits
    pub origin_bits: SpanBatchBits,
    /// Caching - block tx counts
    pub block_tx_counts: Vec<u64>,
    /// Caching - span batch txs
    pub txs: SpanBatchTransactions,
}

impl SpanBatch {
    /// Returns the timestamp for the first batch in the span.
    pub fn get_timestamp(&self) -> u64 {
        self.batches[0].timestamp
    }

    /// Checks if the span batch is valid.
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

        Ok(RawSpanBatch {
            prefix: SpanBatchPrefix {
                rel_timestamp: span_start.timestamp - genesis_timestamp,
                l1_origin_num: span_end.epoch_num,
                parent_check: self.parent_check,
                l1_origin_check: self.l1_origin_check,
            },
            payload: SpanBatchPayload {
                block_count: self.batches.len() as u64,
                origin_bits: self.origin_bits.clone(),
                block_tx_counts: self.block_tx_counts.clone(),
                txs: self.txs.clone(),
            },
        })
    }

    /// Converts all [SpanBatchElement]s after the L2 safe head to [SingleBatch]es. The resulting
    /// [SingleBatch]es do not contain a parent hash, as it is populated by the Batch Queue
    /// stage.
    pub fn get_singular_batches(
        &self,
        l1_origins: Vec<BlockInfo>,
        l2_safe_head: L2BlockInfo,
    ) -> Result<Vec<SingleBatch>, SpanBatchError> {
        let mut single_batches = Vec::new();
        let mut origin_index = 0;
        for batch in &self.batches {
            if batch.timestamp <= l2_safe_head.block_info.timestamp {
                continue;
            }
            let origin_epoch_hash = l1_origins[origin_index..l1_origins.len()]
                .iter()
                .enumerate()
                .find(|(i, origin)| origin.timestamp == batch.timestamp)
                .map(|(i, origin)| {
                    origin_index = i;
                    origin.hash
                })
                .ok_or(SpanBatchError::MissingL1Origin)?;
            let mut single_batch = SingleBatch {
                epoch_num: batch.epoch_num,
                epoch_hash: origin_epoch_hash,
                timestamp: batch.timestamp,
                transactions: batch.transactions.clone(),
                ..Default::default()
            };
            single_batches.push(single_batch);
        }
        Ok(single_batches)
    }

    /// Append a [SingleBatch] to the [SpanBatch]. Updates the L1 origin check if need be.
    pub fn append_singular_batch(
        &mut self,
        singular_batch: SingleBatch,
        seq_num: u64,
    ) -> Result<(), SpanBatchError> {
        // If the new element is not ordered with respect to the last element, panic.
        if !self.batches.is_empty() && self.peek(0).timestamp > singular_batch.timestamp {
            panic!("Batch is not ordered");
        }

        let SingleBatch { epoch_hash, parent_hash, .. } = singular_batch;

        // Always append the new batch and set the L1 origin check.
        self.batches.push(singular_batch.into());
        // Always update the L1 origin check.
        self.l1_origin_check = epoch_hash[..20].try_into().expect("Sub-slice cannot fail");

        let epoch_bit = if self.batches.len() == 1 {
            // If there is only one batch, initialize the parent check and set the epoch bit based
            // on the sequence number.
            self.parent_check = parent_hash[..20].try_into().expect("Sub-slice cannot fail");
            seq_num == 0
        } else {
            // If there is more than one batch, set the epoch bit based on the last two batches.
            self.peek(1).epoch_num < self.peek(0).epoch_num
        };

        // Set the respective bit in the origin bits.
        self.origin_bits.set_bit(self.batches.len() - 1, epoch_bit);

        let new_txs = self.peek(0).transactions.clone();

        // Update the block tx counts cache with the latest batch's transaction count.
        self.block_tx_counts.push(new_txs.len() as u64);

        // Add the new transactions to the transaction cache.
        self.txs.add_txs(new_txs, self.chain_id)
    }

    /// Peek at the `n`th-to-last last element in the batch.
    fn peek(&self, n: usize) -> &SpanBatchElement {
        &self.batches[self.batches.len() - 1 - n]
    }
}
