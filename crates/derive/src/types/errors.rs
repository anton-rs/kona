//! This module contains derivation errors thrown within the pipeline.

use super::SpanBatchError;
use alloy_primitives::B256;
use core::fmt::Display;

/// A result type for the derivation pipeline stages.
pub type StageResult<T> = Result<T, StageError>;

/// An error that is thrown within the stages of the derivation pipeline.
#[derive(Debug)]
pub enum StageError {
    /// There is no data to read from the channel bank.
    Eof,
    /// There is not enough data progress, but if we wait, the stage will eventually return data
    /// or produce an EOF error.
    NotEnoughData,
    /// Failed to build the [super::PayloadAttributes] for the next batch.
    AttributesBuild(anyhow::Error),
    /// Reset the pipeline.
    Reset(ResetError),
    /// The stage detected a block reorg.
    /// The first argument is the expected block hash.
    /// The second argument is the paren_hash of the next l1 origin block.
    ReorgDetected(B256, B256),
    /// Receipt fetching error.
    ReceiptFetch(anyhow::Error),
    /// [super::BlockInfo] fetching error.
    BlockInfoFetch(anyhow::Error),
    /// [super::SystemConfig] update error.
    SystemConfigUpdate(anyhow::Error),
    /// Other wildcard error.
    Custom(anyhow::Error),
}

impl PartialEq<StageError> for StageError {
    fn eq(&self, other: &StageError) -> bool {
        // if it's a reorg detected check the block hashes
        if let (StageError::ReorgDetected(a, b), StageError::ReorgDetected(c, d)) = (self, other) {
            return a == c && b == d;
        }
        if let (StageError::Reset(a), StageError::Reset(b)) = (self, other) {
            return a == b;
        }
        matches!(
            (self, other),
            (StageError::Eof, StageError::Eof) |
                (StageError::NotEnoughData, StageError::NotEnoughData) |
                (StageError::AttributesBuild(_), StageError::AttributesBuild(_)) |
                (StageError::ReceiptFetch(_), StageError::ReceiptFetch(_)) |
                (StageError::BlockInfoFetch(_), StageError::BlockInfoFetch(_)) |
                (StageError::SystemConfigUpdate(_), StageError::SystemConfigUpdate(_)) |
                (StageError::Custom(_), StageError::Custom(_))
        )
    }
}

impl From<anyhow::Error> for StageError {
    fn from(e: anyhow::Error) -> Self {
        StageError::Custom(e)
    }
}

impl Display for StageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StageError::Eof => write!(f, "End of file"),
            StageError::NotEnoughData => write!(f, "Not enough data"),
            StageError::AttributesBuild(e) => write!(f, "Attributes build error: {}", e),
            StageError::Reset(e) => write!(f, "Reset error: {}", e),
            StageError::ReceiptFetch(e) => write!(f, "Receipt fetch error: {}", e),
            StageError::SystemConfigUpdate(e) => write!(f, "System config update error: {}", e),
            StageError::ReorgDetected(current, next) => {
                write!(f, "Block reorg detected: {} -> {}", current, next)
            }
            StageError::BlockInfoFetch(e) => write!(f, "Block info fetch error: {}", e),
            StageError::Custom(e) => write!(f, "Custom error: {}", e),
        }
    }
}

/// A reset error
#[derive(Debug)]
pub enum ResetError {
    /// The batch has a bad parent hash.
    /// The first argument is the expected parent hash, and the second argument is the actual
    /// parent hash.
    BadParentHash(B256, B256),
    /// The batch has a bad timestamp.
    /// The first argument is the expected timestamp, and the second argument is the actual
    /// timestamp.
    BadTimestamp(u64, u64),
}

impl PartialEq<ResetError> for ResetError {
    fn eq(&self, other: &ResetError) -> bool {
        match (self, other) {
            (ResetError::BadParentHash(e1, a1), ResetError::BadParentHash(e2, a2)) => {
                e1 == e2 && a1 == a2
            }
            (ResetError::BadTimestamp(e1, a1), ResetError::BadTimestamp(e2, a2)) => {
                e1 == e2 && a1 == a2
            }
            _ => false,
        }
    }
}

impl Display for ResetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ResetError::BadParentHash(expected, actual) => {
                write!(f, "Bad parent hash: expected {}, got {}", expected, actual)
            }
            ResetError::BadTimestamp(expected, actual) => {
                write!(f, "Bad timestamp: expected {}, got {}", expected, actual)
            }
        }
    }
}

/// A decoding error.
#[derive(Debug)]
pub enum DecodeError {
    /// The buffer is empty.
    EmptyBuffer,
    /// Alloy RLP Encoding Error.
    AlloyRlpError(alloy_rlp::Error),
    /// Span Batch Error.
    SpanBatchError(SpanBatchError),
}

impl From<alloy_rlp::Error> for DecodeError {
    fn from(e: alloy_rlp::Error) -> Self {
        DecodeError::AlloyRlpError(e)
    }
}

impl PartialEq<DecodeError> for DecodeError {
    fn eq(&self, other: &DecodeError) -> bool {
        matches!(
            (self, other),
            (DecodeError::EmptyBuffer, DecodeError::EmptyBuffer) |
                (DecodeError::AlloyRlpError(_), DecodeError::AlloyRlpError(_))
        )
    }
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::EmptyBuffer => write!(f, "Empty buffer"),
            DecodeError::AlloyRlpError(e) => write!(f, "Alloy RLP Decoding Error: {}", e),
            DecodeError::SpanBatchError(e) => write!(f, "Span Batch Decoding Error: {:?}", e),
        }
    }
}
