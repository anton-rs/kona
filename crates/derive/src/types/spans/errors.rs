//! Span Batch Errors

#![allow(dead_code)]

/// Span Batch Errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanBatchError {
    /// The span batch is too big
    TooBigSpanBatchSize,
    /// The bit field is too long
    BitfieldTooLong,
    /// Failed to set [alloy_primitives::U256] from big-endian slice
    InvalidBitSlice,
    /// Encoding errors
    Encoding(EncodingError),
    /// Decoding errors
    Decoding(SpanDecodingError),
}

/// Encoding Error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodingError {
    /// Failed to encode span batch
    SpanBatch,
    /// Failed to encode span batch bits
    SpanBatchBits,
}

/// Decoding Error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanDecodingError {
    /// Failed to decode relative timestamp
    RelativeTimestamp,
    /// Failed to decode L1 origin number
    L1OriginNumber,
    /// Failed to decode parent check
    ParentCheck,
    /// Failed to decode L1 origin check
    L1OriginCheck,
}
