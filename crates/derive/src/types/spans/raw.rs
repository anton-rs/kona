//! Raw Span Batch

use crate::types::SPAN_BATCH_TYPE;
use alloc::vec::Vec;

/// Span Batch Prefix
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchPrefix {
    /// Relative timestamp of the first block
    pub rel_timestamp: u64,
    /// L1 origin number
    pub l1_origin_num: u64,
    /// First 20 bytes of the first block's parent hash
    pub parent_check: [u8; 20],
    /// First 20 bytes of the last block's L1 origin hash
    pub l1_origin_check: [u8; 20],
}

/// Span Batch Payload
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchPayload {
    /// Number of L2 block in the span
    pub block_count: u64,
    /// Standard span-batch bitlist of blockCount bits. Each bit indicates if the L1 origin is changed at the L2 block.
    pub origin_bits: Vec<u8>,
    /// List of transaction counts for each L2 block
    pub block_tx_counts: Vec<u64>,
    /// Transactions encoded in SpanBatch specs
    pub txs: Vec<u8>,
}

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
    pub fn get_batch_type(&self) -> u8 {
        SPAN_BATCH_TYPE
    }
}
