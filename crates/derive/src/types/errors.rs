//! This module contains derivation errors thrown within the pipeline.

use alloy_primitives::B256;
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
