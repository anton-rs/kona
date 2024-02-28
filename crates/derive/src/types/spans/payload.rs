//! Raw Span Batch Payload

use crate::types::spans::{SpanBatchBits, SpanBatchError, SpanDecodingError, MAX_SPAN_BATCH_SIZE};
use alloc::vec::Vec;
use alloy_primitives::U64;
use alloy_rlp::Decodable;

/// Span Batch Payload
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanBatchPayload {
    /// Number of L2 block in the span
    pub block_count: U64,
    /// Standard span-batch bitlist of blockCount bits. Each bit indicates if the L1 origin is changed at the L2 block.
    pub origin_bits: SpanBatchBits,
    /// List of transaction counts for each L2 block
    pub block_tx_counts: Vec<u64>,
    /// Transactions encoded in SpanBatch specs
    pub txs: Vec<u8>,
}

impl SpanBatchPayload {
    /// Decodes the origin bits from a reader.
    pub fn decode_origin_bits(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        self.origin_bits = SpanBatchBits::new(r, self.block_count.to::<u64>() as usize)?;
        Ok(())
    }

    /// Decode a block count from a reader.
    pub fn decode_block_count(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let block_count = U64::decode(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::L1OriginNumber))?;
        if block_count > U64::from(MAX_SPAN_BATCH_SIZE) {
            return Err(SpanBatchError::TooBigSpanBatchSize);
        }
        if block_count.is_zero() {
            return Err(SpanBatchError::EmptyBlockCount);
        }
        self.block_count = block_count;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_rlp::Encodable;

    #[test]
    fn test_decode_block_count() {
        let expected = U64::from(1337);
        let mut buf = Vec::with_capacity(expected.length());
        expected.encode(&mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPayload::default();
        prefix.decode_block_count(&mut r).unwrap();
        assert_eq!(prefix.block_count, expected);
    }
}
