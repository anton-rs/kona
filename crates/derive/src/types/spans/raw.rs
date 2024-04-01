//! Raw Span Batch

use alloc::vec::Vec;

use crate::types::{
    spans::{SpanBatchPayload, SpanBatchPrefix, SpanDecodingError},
    RawTransaction, SpanBatchElement, SPAN_BATCH_TYPE,
};

use super::{SpanBatch, SpanBatchError};

/// Raw Span Batch
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSpanBatch {
    /// The span batch prefix
    pub prefix: SpanBatchPrefix,
    /// The span batch payload
    pub payload: SpanBatchPayload,
}

impl RawSpanBatch {
    /// Encodes the [RawSpanBatch] into a writer.
    pub fn encode(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        self.prefix.encode_prefix(w);
        self.payload.encode_payload(w)
    }

    /// Converts a [RawSpanBatch] into a [SpanBatch], which has a list of [SpanBatchElement]s.
    pub fn derive(
        &mut self,
        block_time: u64,
        genesis_time: u64,
        chain_id: u64,
    ) -> Result<SpanBatch, SpanBatchError> {
        if self.payload.block_count == 0 {
            return Err(SpanBatchError::EmptySpanBatch);
        }

        let mut block_origin_nums = Vec::with_capacity(self.payload.block_count as usize);
        let mut l1_origin_number = self.prefix.l1_origin_num;
        for i in (0..self.payload.block_count).rev() {
            block_origin_nums.push(l1_origin_number);
            if self
                .payload
                .origin_bits
                .get_bit(i as usize)
                .ok_or(SpanBatchError::Decoding(SpanDecodingError::L1OriginCheck))?
                == 1
                && i > 0
            {
                l1_origin_number -= 1;
            }
        }

        // Recover `v` values in transaction signatures within the batch.
        self.payload.txs.recover_v(chain_id)?;

        // Get all transactions in the batch.
        let enveloped_txs = self.payload.txs.full_txs(chain_id)?;

        let mut tx_idx = 0;
        let batches = (0..self.payload.block_count).fold(Vec::new(), |mut acc, i| {
            let transactions =
                (0..self.payload.block_tx_counts[i as usize]).fold(Vec::new(), |mut acc, _| {
                    acc.push(enveloped_txs[tx_idx].clone());
                    tx_idx += 1;
                    acc
                });
            acc.push(SpanBatchElement {
                epoch_num: block_origin_nums[i as usize],
                timestamp: genesis_time + self.prefix.rel_timestamp + block_time * i,
                transactions: transactions.into_iter().map(RawTransaction).collect(),
            });
            acc
        });

        Ok(SpanBatch {
            parent_check: self.prefix.parent_check,
            l1_origin_check: self.prefix.l1_origin_check,
            batches,
        })
    }

    /// Returns the batch type
    pub fn get_batch_type(&self) -> u8 {
        SPAN_BATCH_TYPE
    }
}
