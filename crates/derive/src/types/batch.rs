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

impl TryFrom<&[u8]> for Batch {
    type Error = DecodeError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut buf = bytes;
        if buf.is_empty() {
            return Err(Self::Error::EmptyBuffer);
        }
        match BatchType::from(buf[0]) {
            BatchType::Single => {
                let single_batch = SingleBatch::decode(&mut buf)?;
                Ok(Batch::Single(single_batch))
            }
            BatchType::Span => {
                // TODO: implement span batch decoding
                unimplemented!()
            }
        }
    }
}
