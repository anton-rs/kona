//! This module contains the enumerable [Batch].

use super::batch_type::BatchType;
use super::single_batch::SingleBatch;
use crate::types::errors::DecodeError;

use alloy_rlp::Decodable;

// TODO: replace this with a span batch
/// Span Batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatch {}

/// A Batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Batch {
    /// A single batch
    Single(SingleBatch),
    /// Span Batches
    Span(SpanBatch),
}

impl Batch {
    /// Attempts to decode a batch from a byte slice.
    pub fn decode(r: &mut &[u8]) -> Result<Self, DecodeError> {
        if r.is_empty() {
            return Err(DecodeError::EmptyBuffer);
        }
        match BatchType::from(r[0]) {
            BatchType::Single => {
                let single_batch = SingleBatch::decode(r)?;
                Ok(Batch::Single(single_batch))
            }
            BatchType::Span => {
                // TODO: implement span batch decoding
                unimplemented!()
            }
        }
    }
}
