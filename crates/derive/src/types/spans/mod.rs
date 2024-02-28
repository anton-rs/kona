//! Contains all Span Batch Logic

/// The span batch type
pub const SPAN_BATCH_TYPE: u8 = 0x01;

/// The maximum amount of bytes that will be needed to decode every span
/// batch field. This value cannot be larger than [MAX_RLP_BYTES_PER_CHANNEL]
/// because single batch cannot be larger than channel size.
pub const MAX_SPAN_BATCH_SIZE: usize = MAX_RLP_BYTES_PER_CHANNEL;

/// The maximum amount of bytes that will be read from
/// a channel. This limit is set when decoding the RLP.
pub const MAX_RLP_BYTES_PER_CHANNEL: usize = 10_000_000;

mod bits;
pub use bits::SpanBatchBits;

mod batch;
pub use batch::SpanBatch;

mod errors;
pub use errors::SpanBatchError;

mod raw;
pub use raw::{RawSpanBatch, SpanBatchPayload, SpanBatchPrefix};

mod element;
pub use element::SpanBatchElement;

mod builder;
pub use builder::SpanBatchBuilder;
