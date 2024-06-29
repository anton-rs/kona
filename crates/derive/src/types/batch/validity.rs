//! Contains the [BatchValidity] and its encodings.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Batch Validity
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchValidity {
    /// The batch is invalid now and in the future, unless we reorg
    Drop,
    /// The batch is valid and should be processed
    Accept,
    /// We are lacking L1 information until we can proceed batch filtering
    Undecided,
    /// The batch may be valid, but cannot be processed yet and should be checked again later
    Future,
}

impl BatchValidity {
    /// Returns if the batch is dropped.
    pub fn is_drop(&self) -> bool {
        matches!(self, BatchValidity::Drop)
    }
}
