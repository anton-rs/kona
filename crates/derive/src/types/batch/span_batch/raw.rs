//! Raw Span Batch

use alloc::vec::Vec;

use crate::{
    traits::SafeBlockFetcher,
    types::{
        BatchType, BatchValidity, BlockInfo, L2BlockRef, RawTransaction, RollupConfig, SingleBatch,
        SpanBatchElement, SpanBatchPayload, SpanBatchPrefix, SpanDecodingError,
    },
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
    /// Returns the batch type
    pub fn get_batch_type(&self) -> BatchType {
        BatchType::Span
    }

    /// Returns the timestamp for the span batch.
    pub fn timestamp(&self) -> u64 {
        self.prefix.rel_timestamp
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

    /// Derives [SingleBatch]s from the span batch.
    pub fn get_singular_batches(
        &self,
        _l1_blocks: &[BlockInfo],
        _parent: L2BlockRef,
    ) -> Vec<SingleBatch> {
        unimplemented!()
    }

    /// Encodes the [RawSpanBatch] into a writer.
    pub fn encode(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        self.prefix.encode_prefix(w);
        self.payload.encode_payload(w)
    }

    /// Decodes the [RawSpanBatch] from a reader.]
    pub fn decode(r: &mut &[u8]) -> Result<Self, SpanBatchError> {
        let prefix = SpanBatchPrefix::decode_prefix(r)?;
        let payload = SpanBatchPayload::decode_payload(r)?;
        Ok(Self { prefix, payload })
    }

    /// Converts a [RawSpanBatch] into a [SpanBatch], which has a list of [SpanBatchElement]s. Thos
    /// function does not populate the [SpanBatch] with chain configuration data, which is
    /// required for making payload attributes.
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
                .ok_or(SpanBatchError::Decoding(SpanDecodingError::L1OriginCheck))? ==
                1 &&
                i > 0
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
                transactions: transactions.into_iter().map(|v| RawTransaction(v.into())).collect(),
            });
            acc
        });

        Ok(SpanBatch {
            parent_check: self.prefix.parent_check,
            l1_origin_check: self.prefix.l1_origin_check,
            batches,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use super::RawSpanBatch;
    use alloc::vec::Vec;

    #[test]
    fn test_decode_encode_raw_span_batch() {
        // Load in the raw span batch from the `op-node` derivation pipeline implementation.
        let raw_span_batch_hex = include_bytes!("../../../../testdata/raw_batch.hex");
        let mut raw_span_batch = RawSpanBatch::decode(&mut raw_span_batch_hex.as_slice()).unwrap();
        raw_span_batch.payload.txs.recover_v(981).unwrap();

        let mut encoding_buf = Vec::new();
        raw_span_batch.encode(&mut encoding_buf).unwrap();
        assert_eq!(encoding_buf, raw_span_batch_hex);
    }
}
