//! Errors for the `kona-executor` crate.

use alloc::string::String;
use kona_mpt::TrieNodeError;
use revm::primitives::EVMError;

/// The error type for the [StatelessL2BlockExecutor].
///
/// [StatelessL2BlockExecutor]: crate::StatelessL2BlockExecutor
#[derive(derive_more::Display, Debug)]
pub enum ExecutorError {
    /// Missing gas limit in the payload attributes.
    #[display("Gas limit not provided in payload attributes")]
    MissingGasLimit,
    /// Missing transactions in the payload attributes.
    #[display("Transactions not provided in payload attributes")]
    MissingTransactions,
    /// Missing EIP-1559 parameters in execution payload post-Holocene.
    #[display("Missing EIP-1559 parameters in execution payload post-Holocene")]
    MissingEIP1559Params,
    /// Missing parent beacon block root in the payload attributes.
    #[display("Parent beacon block root not provided in payload attributes")]
    MissingParentBeaconBlockRoot,
    /// Invalid `extraData` field in the block header.
    #[display("Invalid `extraData` field in the block header")]
    InvalidExtraData,
    /// Block gas limit exceeded.
    #[display("Block gas limit exceeded")]
    BlockGasLimitExceeded,
    /// Unsupported transaction type.
    #[display("Unsupported transaction type: {_0}")]
    UnsupportedTransactionType(u8),
    /// Trie DB error.
    #[display("Trie error: {_0}")]
    TrieDBError(TrieDBError),
    /// Execution error.
    #[display("Execution error: {_0}")]
    ExecutionError(EVMError<TrieDBError>),
    /// Signature error.
    #[display("Signature error: {_0}")]
    SignatureError(alloy_primitives::SignatureError),
    /// RLP error.
    #[display("RLP error: {_0}")]
    RLPError(alloy_eips::eip2718::Eip2718Error),
}

impl From<TrieDBError> for ExecutorError {
    fn from(err: TrieDBError) -> Self {
        Self::TrieDBError(err)
    }
}

impl From<TrieNodeError> for ExecutorError {
    fn from(err: TrieNodeError) -> Self {
        Self::TrieDBError(TrieDBError::TrieNode(err))
    }
}

impl core::error::Error for ExecutorError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::TrieDBError(err) => Some(err),
            _ => None,
        }
    }
}

/// A [Result] type for the [ExecutorError] enum.
pub type ExecutorResult<T> = Result<T, ExecutorError>;

/// A [Result] type alias where the error is [TrieDBError].
pub type TrieDBResult<T> = Result<T, TrieDBError>;

/// An error type for [TrieDB] operations.
///
/// [TrieDB]: crate::TrieDB
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
pub enum TrieDBError {
    /// Trie root node has not been blinded.
    #[display("Trie root node has not been blinded")]
    RootNotBlinded,
    /// Missing account info for bundle account.
    #[display("Missing account info for bundle account.")]
    MissingAccountInfo,
    /// Trie node error.
    #[display("Trie node error: {_0}")]
    TrieNode(TrieNodeError),
    /// Trie provider error.
    #[display("Trie provider error: {_0}")]
    Provider(String),
}

impl core::error::Error for TrieDBError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::TrieNode(err) => Some(err),
            _ => None,
        }
    }
}

impl From<TrieNodeError> for TrieDBError {
    fn from(err: TrieNodeError) -> Self {
        Self::TrieNode(err)
    }
}
