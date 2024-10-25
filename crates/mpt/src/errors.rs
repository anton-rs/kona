//! Errors for the `kona-derive` crate.

use alloc::string::String;

/// A [Result] type alias where the error is [TrieNodeError].
pub type TrieNodeResult<T> = Result<T, TrieNodeError>;

/// An error type for [TrieNode] operations.
///
/// [TrieNode]: crate::TrieNode
#[derive(Debug, derive_more::Display, PartialEq, Eq)]
pub enum TrieNodeError {
    /// Invalid trie node type encountered.
    #[display("Invalid trie node type encountered")]
    InvalidNodeType,
    /// Failed to decode trie node.
    #[display("Failed to decode trie node: {_0}")]
    RLPError(alloy_rlp::Error),
    /// Key does not exist in trie.
    #[display("Key does not exist in trie. Encountered {_0} node.")]
    KeyNotFound(String),
    /// Trie node is not a leaf node.
    #[display("Trie provider error: {_0}")]
    Provider(String),
}

impl core::error::Error for TrieNodeError {}

/// A [Result] type alias where the error is [OrderedListWalkerError].
pub type OrderedListWalkerResult<T> = Result<T, OrderedListWalkerError>;

/// An error type for [OrderedListWalker] operations.
///
/// [OrderedListWalker]: crate::OrderedListWalker
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
pub enum OrderedListWalkerError {
    /// Iterator has already been hydrated, and cannot be re-hydrated until it is exhausted.
    #[display(
        "Iterator has already been hydrated, and cannot be re-hydrated until it is exhausted"
    )]
    AlreadyHydrated,
    /// Trie node error.
    #[display("{_0}")]
    TrieNode(TrieNodeError),
}

impl From<TrieNodeError> for OrderedListWalkerError {
    fn from(err: TrieNodeError) -> Self {
        Self::TrieNode(err)
    }
}

impl core::error::Error for OrderedListWalkerError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::TrieNode(err) => Some(err),
            _ => None,
        }
    }
}
