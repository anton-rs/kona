//! Error types for the `kona-interop` crate.

use alloy_primitives::{Address, B256};
use thiserror::Error;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DependencyGraphError {
    #[error("No blocks passed")]
    EmptyDependencySet,
    #[error("Headers have mismatched timestamps")]
    MismatchedTimestamps,
    #[error("Interop provider: {0}")]
    InteropProviderError(#[from] InteropProviderError),
    #[error("Remote message not found on chain ID {0} with message hash {1}")]
    RemoteMessageNotFound(u64, B256),
    #[error("Unknown Chain ID: {0}")]
    UnknownChainId(u64),
    #[error("Invalid message origin. Expected {0}, got {1}")]
    InvalidMessageOrigin(Address, Address),
    #[error("Invalid message hash. Expected {0}, got {1}")]
    InvalidMessageHash(B256, B256),
    #[error("Invalid message timestamp. Expected {0}, got {1}")]
    InvalidMessageTimestamp(u64, u64),
    #[error("Message is in the future. Expected timestamp to be <= {0}, got {1}")]
    MessageInFuture(u64, u64),
    #[error("Message has already been executed")]
    InvalidMessages(Vec<u64>),
}

/// A [Result] alias for the [DependencyGraphError] type.
pub type DependencyGraphResult<T> = core::result::Result<T, DependencyGraphError>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InteropProviderError {
    #[error("Unknown Chain ID")]
    UnknownChainId,
    #[error("Not found")]
    NotFound,
}

/// A [Result] alias for the [InteropProviderError] type.
pub type InteropProviderResult<T> = core::result::Result<T, InteropProviderError>;
