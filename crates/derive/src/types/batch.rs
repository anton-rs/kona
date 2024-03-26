//! This module contains the enumerable [Batch].

use super::batch_type::BatchType;
use super::single_batch::SingleBatch;

use alloy_rlp::{Decodable, Encodable};

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

impl Decodable for Batch {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        // The buffer must have at least one identifier byte.
        if buf.is_empty() {
            return Err(alloy_rlp::Error::Custom("Empty buffer"));
        }
        match BatchType::from(buf[0]) {
            BatchType::Single => {
                let single_batch = SingleBatch::decode(buf)?;
                Ok(Batch::Single(single_batch))
            }
            BatchType::Span => {
                // TODO: implement span batch decoding
                unimplemented!()
            }
        } 
    }
}

impl Encodable for Batch {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Batch::Single(single_batch) => {
                BatchType::Single.encode(out);
                single_batch.encode(out);
            }
            Batch::Span(_) => {
                // TODO: implement span batch encoding
                unimplemented!()
            }
        }
    }
}


