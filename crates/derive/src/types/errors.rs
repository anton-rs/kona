//! This module contains derivation errors thrown within the pipeline.

use super::Frame;
use alloc::vec::Vec;
use alloy_primitives::{Bytes, B256};
use core::fmt::Display;

/// An error that is thrown within the stages of the derivation pipeline.
#[derive(Debug)]
pub enum StageError {
    /// There is no data to read from the channel bank.
    Eof,
    /// There is not enough data progress, but if we wait, the stage will eventually return data
    /// or produce an EOF error.
    NotEnoughData,
    /// Failed to fetch block info and transactions by hash.
    BlockFetch(B256),
    /// No item returned from the previous stage iterator.
    Empty,
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

/// Converts a stage result into a vector of frames.
pub fn into_frames<T: Into<Bytes>>(result: StageResult<T>) -> anyhow::Result<Vec<Frame>> {
    match result {
        Ok(data) => Ok(Frame::parse_frames(&data.into())?),
        Err(e) => Err(anyhow::anyhow!(e)),
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
            StageError::BlockFetch(hash) => write!(
                f,
                "Failed to fetch block info and transactions by hash: {}",
                hash
            ),
            StageError::Empty => write!(f, "Empty"),
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
        }
    }
}
