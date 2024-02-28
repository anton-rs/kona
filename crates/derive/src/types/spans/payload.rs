//! Raw Span Batch Payload

use crate::types::spans::{SpanBatchBits, SpanBatchError};
use alloc::vec::Vec;

/// Span Batch Payload
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchPayload {
    /// Number of L2 block in the span
    pub block_count: u64,
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
        self.origin_bits = SpanBatchBits::new(r, self.block_count as usize)?;
        Ok(())
    }
}
