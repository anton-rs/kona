//! This module contains derivation errors thrown within the pipeline.

use core::fmt::Display;

use super::SpanBatchError;

/// An error that is thrown within the stages of the derivation pipeline.
#[derive(Debug)]
pub enum StageError {
    /// There is no data to read from the channel bank.
    Eof,
    /// There is not enough data progress, but if we wait, the stage will eventually return data
    /// or produce an EOF error.
    NotEnoughData,
    /// Other wildcard error.
    Custom(anyhow::Error),
}

impl PartialEq<StageError> for StageError {
    fn eq(&self, other: &StageError) -> bool {
        matches!(
            (self, other),
            (StageError::Eof, StageError::Eof)
                | (StageError::NotEnoughData, StageError::NotEnoughData)
                | (StageError::Custom(_), StageError::Custom(_))
        )
    }
}

/// A result type for the derivation pipeline stages.
pub type StageResult<T> = Result<T, StageError>;

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
            StageError::Custom(e) => write!(f, "Custom error: {}", e),
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
            (DecodeError::EmptyBuffer, DecodeError::EmptyBuffer)
                | (DecodeError::AlloyRlpError(_), DecodeError::AlloyRlpError(_))
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
