//! Raw Span Batch

use crate::types::{
    spans::{SpanBatchPayload, SpanBatchPrefix},
    SPAN_BATCH_TYPE,
};

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
