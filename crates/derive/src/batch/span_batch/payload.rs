//! Raw Span Batch Payload

use super::{FJORD_MAX_SPAN_BATCH_BYTES, MAX_SPAN_BATCH_BYTES};
use crate::batch::{SpanBatchBits, SpanBatchError, SpanBatchTransactions, SpanDecodingError};
use alloc::vec::Vec;

/// Span Batch Payload
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanBatchPayload {
    /// Number of L2 block in the span
    pub block_count: u64,
    /// Standard span-batch bitlist of blockCount bits. Each bit indicates if the L1 origin is
    /// changed at the L2 block.
    pub origin_bits: SpanBatchBits,
    /// List of transaction counts for each L2 block
    pub block_tx_counts: Vec<u64>,
    /// Transactions encoded in SpanBatch specs
    pub txs: SpanBatchTransactions,
}

impl SpanBatchPayload {
    /// Decodes a [SpanBatchPayload] from a reader.
    pub fn decode_payload(r: &mut &[u8], is_fjord_active: bool) -> Result<Self, SpanBatchError> {
        let mut payload = Self::default();
        payload.decode_block_count(r, is_fjord_active)?;
        payload.decode_origin_bits(r, is_fjord_active)?;
        payload.decode_block_tx_counts(r, is_fjord_active)?;
        payload.decode_txs(r, is_fjord_active)?;
        Ok(payload)
    }

    /// Encodes a [SpanBatchPayload] into a writer.
    pub fn encode_payload(
        &self,
        w: &mut Vec<u8>,
        is_fjord_active: bool,
    ) -> Result<(), SpanBatchError> {
        self.encode_block_count(w);
        self.encode_origin_bits(w, is_fjord_active)?;
        self.encode_block_tx_counts(w);
        self.encode_txs(w, is_fjord_active)
    }

    /// Decodes the origin bits from a reader.
    pub fn decode_origin_bits(
        &mut self,
        r: &mut &[u8],
        is_fjord_active: bool,
    ) -> Result<(), SpanBatchError> {
        self.origin_bits = SpanBatchBits::decode(r, self.block_count as usize, is_fjord_active)?;
        Ok(())
    }

    /// Returns the max span batch size based on the Fjord hardfork.
    pub const fn max_span_batch_size(&self, is_fjord_active: bool) -> usize {
        if is_fjord_active {
            FJORD_MAX_SPAN_BATCH_BYTES as usize
        } else {
            MAX_SPAN_BATCH_BYTES as usize
        }
    }

    /// Decode a block count from a reader.
    pub fn decode_block_count(
        &mut self,
        r: &mut &[u8],
        is_fjord_active: bool,
    ) -> Result<(), SpanBatchError> {
        let (block_count, remaining) = unsigned_varint::decode::u64(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::BlockCount))?;
        // The number of transactions in a single L2 block cannot be greater than
        // [MAX_SPAN_BATCH_BYTES] or [FJORD_MAX_SPAN_BATCH_BYTES] if Fjord is active.
        let max_span_batch_size = self.max_span_batch_size(is_fjord_active);
        if block_count as usize > max_span_batch_size {
            return Err(SpanBatchError::TooBigSpanBatchSize);
        }
        if block_count == 0 {
            return Err(SpanBatchError::EmptySpanBatch);
        }
        self.block_count = block_count;
        *r = remaining;
        Ok(())
    }

    /// Decode block transaction counts from a reader.
    pub fn decode_block_tx_counts(
        &mut self,
        r: &mut &[u8],
        is_fjord_active: bool,
    ) -> Result<(), SpanBatchError> {
        // Initially allocate the vec with the block count, to reduce re-allocations in the first
        // few blocks.
        let mut block_tx_counts = Vec::with_capacity(self.block_count as usize);

        for _ in 0..self.block_count {
            let (block_tx_count, remaining) = unsigned_varint::decode::u64(r)
                .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::BlockTxCounts))?;

            // The number of transactions in a single L2 block cannot be greater than
            // [MAX_SPAN_BATCH_BYTES] or [FJORD_MAX_SPAN_BATCH_BYTES] if Fjord is active.
            // Every transaction will take at least a single byte.
            let max_span_batch_size = self.max_span_batch_size(is_fjord_active);
            if block_tx_count as usize > max_span_batch_size {
                return Err(SpanBatchError::TooBigSpanBatchSize);
            }
            block_tx_counts.push(block_tx_count);
            *r = remaining;
        }
        self.block_tx_counts = block_tx_counts;
        Ok(())
    }

    /// Decode transactions from a reader.
    pub fn decode_txs(
        &mut self,
        r: &mut &[u8],
        is_fjord_active: bool,
    ) -> Result<(), SpanBatchError> {
        if self.block_tx_counts.is_empty() {
            return Err(SpanBatchError::EmptySpanBatch);
        }

        let total_block_tx_count =
            self.block_tx_counts.iter().try_fold(0u64, |acc, block_tx_count| {
                acc.checked_add(*block_tx_count).ok_or(SpanBatchError::TooBigSpanBatchSize)
            })?;

        // The total number of transactions in a span batch cannot be greater than
        // [MAX_SPAN_BATCH_BYTES] or [FJORD_MAX_SPAN_BATCH_BYTES] if Fjord is active.
        let max_span_batch_size = self.max_span_batch_size(is_fjord_active);
        if total_block_tx_count as usize > max_span_batch_size {
            return Err(SpanBatchError::TooBigSpanBatchSize);
        }
        self.txs.total_block_tx_count = total_block_tx_count;
        self.txs.decode(r, is_fjord_active)?;
        Ok(())
    }

    /// Encode the origin bits into a writer.
    pub fn encode_origin_bits(
        &self,
        w: &mut Vec<u8>,
        is_fjord_active: bool,
    ) -> Result<(), SpanBatchError> {
        SpanBatchBits::encode(w, self.block_count as usize, &self.origin_bits, is_fjord_active)
    }

    /// Encode the block count into a writer.
    pub fn encode_block_count(&self, w: &mut Vec<u8>) {
        let mut u64_varint_buf = [0u8; 10];
        w.extend_from_slice(unsigned_varint::encode::u64(self.block_count, &mut u64_varint_buf));
    }

    /// Encode the block transaction counts into a writer.
    pub fn encode_block_tx_counts(&self, w: &mut Vec<u8>) {
        let mut u64_varint_buf = [0u8; 10];
        for block_tx_count in &self.block_tx_counts {
            u64_varint_buf.fill(0);
            w.extend_from_slice(unsigned_varint::encode::u64(*block_tx_count, &mut u64_varint_buf));
        }
    }

    /// Encode the transactions into a writer.
    pub fn encode_txs(&self, w: &mut Vec<u8>, is_fjord_active: bool) -> Result<(), SpanBatchError> {
        self.txs.encode(w, is_fjord_active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_decode_origin_bits() {
        let block_count = 10;
        let encoded = vec![2; block_count / 8 + 1];
        let mut payload =
            SpanBatchPayload { block_count: block_count as u64, ..Default::default() };
        payload.decode_origin_bits(&mut encoded.as_slice(), false).unwrap();
        assert_eq!(payload.origin_bits, SpanBatchBits(vec![2; block_count / 8 + 1]));
    }

    #[test]
    fn test_zero_block_count() {
        let mut u64_varint_buf = [0; 10];
        let mut encoded = unsigned_varint::encode::u64(0, &mut u64_varint_buf);
        let mut payload = SpanBatchPayload::default();
        let err = payload.decode_block_count(&mut encoded, false).unwrap_err();
        assert_eq!(err, SpanBatchError::EmptySpanBatch);
    }

    #[test]
    fn test_decode_block_count_pre_fjord() {
        let block_count = MAX_SPAN_BATCH_BYTES;
        let mut u64_varint_buf = [0; 10];
        let mut encoded = unsigned_varint::encode::u64(block_count, &mut u64_varint_buf);
        let mut payload = SpanBatchPayload::default();
        payload.decode_block_count(&mut encoded, false).unwrap();
        assert_eq!(payload.block_count, block_count);
    }

    #[test]
    fn test_decode_block_count_pre_fjord_errors() {
        let block_count = MAX_SPAN_BATCH_BYTES + 1;
        let mut u64_varint_buf = [0; 10];
        let mut encoded = unsigned_varint::encode::u64(block_count, &mut u64_varint_buf);
        let mut payload = SpanBatchPayload::default();
        let err = payload.decode_block_count(&mut encoded, false).unwrap_err();
        assert_eq!(err, SpanBatchError::TooBigSpanBatchSize);
    }

    #[test]
    fn test_decode_block_count_post_fjord() {
        let block_count = FJORD_MAX_SPAN_BATCH_BYTES;
        let mut u64_varint_buf = [0; 10];
        let mut encoded = unsigned_varint::encode::u64(block_count, &mut u64_varint_buf);
        let mut payload = SpanBatchPayload::default();
        payload.decode_block_count(&mut encoded, true).unwrap();
        assert_eq!(payload.block_count, block_count);
    }

    #[test]
    fn test_decode_block_count_post_fjord_errors() {
        let block_count = FJORD_MAX_SPAN_BATCH_BYTES + 1;
        let mut u64_varint_buf = [0; 10];
        let mut encoded = unsigned_varint::encode::u64(block_count, &mut u64_varint_buf);
        let mut payload = SpanBatchPayload::default();
        let err = payload.decode_block_count(&mut encoded, true).unwrap_err();
        assert_eq!(err, SpanBatchError::TooBigSpanBatchSize);
    }

    #[test]
    fn test_decode_block_tx_counts() {
        let block_count = 2;
        let mut u64_varint_buf = [0; 10];
        let mut encoded = unsigned_varint::encode::u64(block_count, &mut u64_varint_buf);
        let mut payload = SpanBatchPayload::default();
        payload.decode_block_count(&mut encoded, false).unwrap();
        let mut r: Vec<u8> = Vec::new();
        for _ in 0..2 {
            let mut buf = [0u8; 10];
            let encoded = unsigned_varint::encode::u64(2, &mut buf);
            r.append(&mut encoded.to_vec());
        }
        payload.decode_block_tx_counts(&mut r.as_slice(), false).unwrap();
        assert_eq!(payload.block_tx_counts, vec![2, 2]);
    }

    #[test]
    fn test_max_span_batch_size() {
        let payload = SpanBatchPayload::default();
        assert_eq!(payload.max_span_batch_size(false), MAX_SPAN_BATCH_BYTES as usize);
        assert_eq!(payload.max_span_batch_size(true), FJORD_MAX_SPAN_BATCH_BYTES as usize);
    }
}
