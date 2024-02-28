//! Span Batch Errors

/// Span Batch Errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanBatchError {
    /// The span batch is too big
    TooBigSpanBatchSize,
    /// The bit field is too long
    BitfieldTooLong,
    /// Failed to set [alloy_primitives::U256] from big-endian slice
    InvalidBitSlice,
}
