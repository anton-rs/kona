//! Raw Span Batch Payload

use crate::types::spans::{SpanBatchBits, SpanBatchTransactions, SpanBatchError, SpanDecodingError, MAX_SPAN_BATCH_SIZE};
use alloc::vec::Vec;
use alloy_rlp::Decodable;

/// Span Batch Payload
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanBatchPayload {
    /// Number of L2 block in the span
    pub block_count: u64,
    /// Standard span-batch bitlist of blockCount bits. Each bit indicates if the L1 origin is changed at the L2 block.
    pub origin_bits: SpanBatchBits,
    /// List of transaction counts for each L2 block
    pub block_tx_counts: Vec<u64>,
    /// Transactions encoded in SpanBatch specs
    pub txs: SpanBatchTransactions,
}

impl SpanBatchPayload {
    /// Decodes the origin bits from a reader.
    pub fn decode_origin_bits(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        self.origin_bits = SpanBatchBits::new(r, self.block_count as usize)?;
        Ok(())
    }

    /// Decode a block count from a reader.
    pub fn decode_block_count(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let (block_count, _) = unsigned_varint::decode::u64(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::L1OriginNumber))?;
        if block_count as usize > MAX_SPAN_BATCH_SIZE {
            return Err(SpanBatchError::TooBigSpanBatchSize);
        }
        if block_count == 0 {
            return Err(SpanBatchError::EmptyBlockCount);
        }
        self.block_count = block_count;
        Ok(())
    }

    /// Decodes the block tx counts from a reader.
    pub fn decode_block_tx_counts(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let block_tx_counts = Vec::<u64>::decode(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::RelativeTimestamp))?;
        for b in block_tx_counts[..].iter() {
            if *b as usize > MAX_SPAN_BATCH_SIZE {
                return Err(SpanBatchError::TooBigSpanBatchSize);
            }
        }
        self.block_tx_counts = block_tx_counts;
        Ok(())
    }

    /// Decodes the transactions from raw bytes.
    pub fn decode_transactions(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        // Validate the cumulative block tx counts does not exceed the maximum span batch size.
        let mut total_txs = 0;
        for b in self.block_tx_counts.iter() {
            total_txs += *b;
            if total_txs as usize > MAX_SPAN_BATCH_SIZE {
                return Err(SpanBatchError::TooBigSpanBatchSize);
            }
        }
        self.txs = SpanBatchTransactions::decode(r).map_err(|_| SpanBatchError::Decoding(SpanDecodingError::Transactions))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_decode_origin_bits() {
        let buf = [12; 1];
        let mut r = &buf[..];
        let mut prefix = SpanBatchPayload { block_count: 8u64, ..Default::default() };
        assert_eq!(prefix.decode_origin_bits(&mut r), Ok(()));
        assert_eq!(prefix.origin_bits, SpanBatchBits(vec![12]));
    }

    /// Generates test targets for decoding the block count
    macro_rules! decoding_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (expected, res) = $value;
                let mut buf = [0u8; 10];
                unsigned_varint::encode::u64(expected, &mut buf);
                let mut r = &buf[..];
                let mut prefix = SpanBatchPayload::default();
                assert_eq!(prefix.decode_block_count(&mut r), res);
                if res.is_ok() {
                    assert_eq!(prefix.block_count, expected);
                }
            }
        )*
        }
    }
    decoding_tests! {
        test_decode_block_count: (1337u64, Ok(())),
        test_decode_block_count_zero: (0u64, Err(SpanBatchError::EmptyBlockCount)),
        test_decode_block_count_max: (MAX_SPAN_BATCH_SIZE as u64, Ok(())),
        test_decode_block_count_too_big: ((MAX_SPAN_BATCH_SIZE + 1) as u64, Err(SpanBatchError::TooBigSpanBatchSize)),
    }

    /// Generates test targets for decoding the block tx counts
    macro_rules! decoding_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                use alloy_rlp::Encodable;
                let (expected, res) = $value;
                let mut buf = alloc::vec::Vec::with_capacity(expected.length());
                expected.encode(&mut buf);
                let mut r = &buf[..];
                let mut prefix = SpanBatchPayload::default();
                assert_eq!(prefix.decode_block_tx_counts(&mut r), res);
                if res.is_ok() {
                    assert_eq!(prefix.block_tx_counts, expected);
                }
            }
        )*
        }
    }
    decoding_tests! {
        test_decode_block_tx_counts: (vec![0u64, 1u64, 2u64], Ok(())),
        test_decode_block_tx_counts_max: (vec![MAX_SPAN_BATCH_SIZE as u64], Ok(())),
        test_decode_block_tx_counts_too_big: (vec![(MAX_SPAN_BATCH_SIZE + 1) as u64], Err(SpanBatchError::TooBigSpanBatchSize)),
    }
}
