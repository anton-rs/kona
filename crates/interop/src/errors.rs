//! Error types for the `kona-interop` crate.

use alloc::vec::Vec;
use alloy_primitives::{Address, B256};
use thiserror::Error;

/// An error type for the [MessageGraph] struct.
///
/// [MessageGraph]: crate::MessageGraph
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MessageGraphError {
    /// Dependency set is impossibly empty
    #[error("Dependency set is impossibly empty")]
    EmptyDependencySet,
    /// Remote message not found
    #[error("Remote message not found on chain ID {0} with message hash {1}")]
    RemoteMessageNotFound(u64, B256),
    /// Invalid message origin
    #[error("Invalid message origin. Expected {0}, got {1}")]
    InvalidMessageOrigin(Address, Address),
    /// Invalid message payload hash
    #[error("Invalid message hash. Expected {0}, got {1}")]
    InvalidMessageHash(B256, B256),
    /// Invalid message timestamp
    #[error("Invalid message timestamp. Expected {0}, got {1}")]
    InvalidMessageTimestamp(u64, u64),
    /// Message is in the future
    #[error("Message is in the future. Expected timestamp to be <= {0}, got {1}")]
    MessageInFuture(u64, u64),
    /// Invalid messages were found
    #[error("Invalid messages found on chains: {0:?}")]
    InvalidMessages(Vec<u64>),
    /// Interop provider error
    #[error("Interop provider: {0}")]
    InteropProviderError(#[from] InteropProviderError),
}

/// A [Result] alias for the [MessageGraphError] type.
pub type MessageGraphResult<T> = core::result::Result<T, MessageGraphError>;

/// An error type for the [InteropProvider] trait.
///
/// [InteropProvider]: crate::InteropProvider
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InteropProviderError {
    /// Unknown Chain ID
    #[error("Unknown Chain ID")]
    UnknownChainId,
    /// Not found
    #[error("Not found")]
    NotFound,
}

/// A [Result] alias for the [InteropProviderError] type.
pub type InteropProviderResult<T> = core::result::Result<T, InteropProviderError>;

/// An error type for the [SuperRoot] struct's serialization and deserialization.
///
/// [SuperRoot]: crate::SuperRoot
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SuperRootError {
    /// Invalid super root version byte
    #[error("Invalid super root version byte")]
    InvalidVersionByte,
    /// Unexpected encoded super root length
    #[error("Unexpected encoded super root length")]
    UnexpectedLength,
}

/// A [Result] alias for the [SuperRootError] type.
pub type SuperRootResult<T> = core::result::Result<T, SuperRootError>;
