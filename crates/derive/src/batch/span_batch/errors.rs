//! Span Batch Errors

use thiserror::Error;

/// Span Batch Errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SpanBatchError {
    /// The span batch is too big
    #[error("The span batch is too big.")]
    TooBigSpanBatchSize,
    /// The bit field is too long
    #[error("The bit field is too long")]
    BitfieldTooLong,
    /// Empty Span Batch
    #[error("Empty span batch")]
    EmptySpanBatch,
    /// Missing L1 origin
    #[error("Missing L1 origin")]
    MissingL1Origin,
    /// Decoding errors
    #[error("Span batch decoding error: {0}")]
    Decoding(#[from] SpanDecodingError),
}

/// Decoding Error
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SpanDecodingError {
    /// Failed to decode relative timestamp
    #[error("Failed to decode relative timestamp")]
    RelativeTimestamp,
    /// Failed to decode L1 origin number
    #[error("Failed to decode L1 origin number")]
    L1OriginNumber,
    /// Failed to decode parent check
    #[error("Failed to decode parent check")]
    ParentCheck,
    /// Failed to decode L1 origin check
    #[error("Failed to decode L1 origin check")]
    L1OriginCheck,
    /// Failed to decode block count
    #[error("Failed to decode block count")]
    BlockCount,
    /// Failed to decode block tx counts
    #[error("Failed to decode block tx counts")]
    BlockTxCounts,
    /// Failed to decode transaction nonces
    #[error("Failed to decode transaction nonces")]
    TxNonces,
    /// Mismatch in length between the transaction type and signature arrays in a span batch
    /// transaction payload.
    #[error("Mismatch in length between the transaction type and signature arrays")]
    TypeSignatureLenMismatch,
    /// Invalid transaction type
    #[error("Invalid transaction type")]
    InvalidTransactionType,
    /// Invalid transaction data
    #[error("Invalid transaction data")]
    InvalidTransactionData,
    /// Invalid transaction signature
    #[error("Invalid transaction signature")]
    InvalidTransactionSignature,
}
