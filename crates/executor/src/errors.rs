//! Errors for the `kona-executor` crate.

use kona_mpt::TrieDBError;
use noerror::Error;
use revm::primitives::EVMError;

/// The error type for the [StatelessL2BlockExecutor].
///
/// [StatelessL2BlockExecutor]: crate::StatelessL2BlockExecutor
#[derive(Error, Debug)]
pub enum ExecutorError {
    /// Missing gas limit in the payload attributes.
    #[error("Gas limit not provided in payload attributes")]
    MissingGasLimit,
    /// Missing EIP-1559 parameters in execution payload post-Holocene.
    #[error("Missing EIP-1559 parameters in execution payload post-Holocene")]
    MissingEIP1559Params,
    /// Missing parent beacon block root in the payload attributes.
    #[error("Parent beacon block root not provided in payload attributes")]
    MissingParentBeaconBlockRoot,
    /// Block gas limit exceeded.
    #[error("Block gas limit exceeded")]
    BlockGasLimitExceeded,
    /// Unsupported transaction type.
    #[error("Unsupported transaction type: {0}")]
    UnsupportedTransactionType(u8),
    /// Trie DB error.
    #[error("Trie error: {0}")]
    TrieDBError(#[from] TrieDBError),
    /// Execution error.
    #[error("Execution error: {0}")]
    ExecutionError(EVMError<TrieDBError>),
    /// Signature error.
    #[error("Signature error: {0}")]
    SignatureError(alloy_primitives::SignatureError),
    /// RLP error.
    #[error("RLP error: {0}")]
    RLPError(alloy_eips::eip2718::Eip2718Error),
}

/// A [Result] type for the [ExecutorError] enum.
pub type ExecutorResult<T> = Result<T, ExecutorError>;
