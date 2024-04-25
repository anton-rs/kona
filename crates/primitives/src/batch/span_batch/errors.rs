//! Span Batch Errors

use core::fmt::Display;

/// Span Batch Errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanBatchError {
    /// The span batch is too big
    TooBigSpanBatchSize,
    /// The bit field is too long
    BitfieldTooLong,
    /// Failed to set [alloy_primitives::U256] from big-endian slice
    InvalidBitSlice,
    /// Empty Span Batch
    EmptySpanBatch,
    /// Missing L1 origin
    MissingL1Origin,
    /// Encoding errors
    Encoding(EncodingError),
    /// Decoding errors
    Decoding(SpanDecodingError),
}

impl Display for SpanBatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpanBatchError::TooBigSpanBatchSize => write!(f, "The span batch is too big"),
            SpanBatchError::BitfieldTooLong => write!(f, "The bit field is too long"),
            SpanBatchError::InvalidBitSlice => {
                write!(f, "Failed to set [alloy_primitives::U256] from big-endian slice")
            }
            SpanBatchError::EmptySpanBatch => write!(f, "Empty Span Batch"),
            SpanBatchError::MissingL1Origin => write!(f, "Missing L1 origin"),
            SpanBatchError::Encoding(e) => write!(f, "Encoding error: {:?}", e),
            SpanBatchError::Decoding(e) => write!(f, "Decoding error: {:?}", e),
        }
    }
}

/// Encoding Error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodingError {
    /// Failed to encode span batch
    SpanBatch,
    /// Failed to encode span batch bits
    SpanBatchBits,
}

impl Display for EncodingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EncodingError::SpanBatch => write!(f, "Failed to encode span batch"),
            EncodingError::SpanBatchBits => write!(f, "Failed to encode span batch bits"),
        }
    }
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
    /// Failed to decode block count
    BlockCount,
    /// Failed to decode block tx counts
    BlockTxCounts,
    /// Failed to decode transaction nonces
    TxNonces,
    /// Mismatch in length between the transaction type and signature arrays in a span batch
    /// transaction payload.
    TypeSignatureLenMismatch,
    /// Invalid transaction type
    InvalidTransactionType,
    /// Invalid transaction data
    InvalidTransactionData,
    /// Invalid transaction signature
    InvalidTransactionSignature,
}

impl Display for SpanDecodingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpanDecodingError::RelativeTimestamp => {
                write!(f, "Failed to decode relative timestamp")
            }
            SpanDecodingError::L1OriginNumber => write!(f, "Failed to decode L1 origin number"),
            SpanDecodingError::ParentCheck => write!(f, "Failed to decode parent check"),
            SpanDecodingError::L1OriginCheck => write!(f, "Failed to decode L1 origin check"),
            SpanDecodingError::BlockCount => write!(f, "Failed to decode block count"),
            SpanDecodingError::BlockTxCounts => write!(f, "Failed to decode block tx counts"),
            SpanDecodingError::TxNonces => write!(f, "Failed to decode transaction nonces"),
            SpanDecodingError::TypeSignatureLenMismatch => {
                write!(f, "Mismatch in length between the transaction type and signature arrays in a span batch transaction payload")
            }
            SpanDecodingError::InvalidTransactionType => write!(f, "Invalid transaction type"),
            SpanDecodingError::InvalidTransactionData => write!(f, "Invalid transaction data"),
            SpanDecodingError::InvalidTransactionSignature => {
                write!(f, "Invalid transaction signature")
            }
        }
    }
}
