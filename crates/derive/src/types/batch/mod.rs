//! This module contains the batch types for the OP Stack derivation pipeline: [SpanBatch] & [SingleBatch].

use super::DecodeError;
use alloc::vec::Vec;
use alloy_rlp::{Buf, Decodable, Encodable};

mod batch_type;
pub use batch_type::BatchType;

mod span_batch;
pub use span_batch::{
    RawSpanBatch, SpanBatch, SpanBatchBits, SpanBatchBuilder, SpanBatchEip1559TransactionData,
    SpanBatchEip2930TransactionData, SpanBatchElement, SpanBatchError,
    SpanBatchLegacyTransactionData, SpanBatchPayload, SpanBatchPrefix, SpanBatchTransactionData,
    SpanBatchTransactions, SpanDecodingError, MAX_SPAN_BATCH_SIZE, SPAN_BATCH_TYPE,
};

mod single_batch;
pub use single_batch::SingleBatch;

/// A Batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Batch {
    /// A single batch
    Single(SingleBatch),
    /// Span Batches
    Span(SpanBatch),
}

impl Batch {
    /// Attempts to encode a batch into a writer.
    pub fn encode(&self, w: &mut Vec<u8>) -> Result<(), DecodeError> {
        match self {
            Self::Single(single_batch) => {
                single_batch.encode(w);
                Ok(())
            }
            Self::Span(_span_batch) => {
                unimplemented!()
            }
        }
    }

    /// Attempts to decode a batch from a reader.
    pub fn decode(r: &mut &[u8]) -> Result<Self, DecodeError> {
        if r.is_empty() {
            return Err(DecodeError::EmptyBuffer);
        }

        // Read the batch type
        let batch_type = BatchType::from(r[0]);
        r.advance(1);

        match batch_type {
            BatchType::Single => {
                let single_batch = SingleBatch::decode(r)?;
                Ok(Batch::Single(single_batch))
            }
            BatchType::Span => {
                unimplemented!()
            }
        }
    }
}
