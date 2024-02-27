//! Contains all Span Batch Logic

/// The span batch type
pub const SPAN_BATCH_TYPE: u8 = 0x01;

/// Span batch type
mod batch;
pub use batch::SpanBatch;

mod raw;
pub use raw::{RawSpanBatch, SpanBatchPayload, SpanBatchPrefix};

/// Span batch element type
mod element;
pub use element::SpanBatchElement;

/// Span batch builder type
mod builder;
pub use builder::SpanBatchBuilder;
