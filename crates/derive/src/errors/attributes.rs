//! Error types for kona's attributes builder.

use alloc::string::String;
use alloy_eips::BlockNumHash;
use alloy_primitives::B256;

/// An [AttributesBuilder] Error.
///
/// [AttributesBuilder]: crate::traits::AttributesBuilder
#[derive(derive_more::Display, Clone, Debug, PartialEq, Eq)]
pub enum BuilderError {
    /// Mismatched blocks.
    #[display("Block mismatch. Expected {_0:?}, got {_1:?}")]
    BlockMismatch(BlockNumHash, BlockNumHash),
    /// Mismatched blocks for the start of an Epoch.
    #[display("Block mismatch on epoch reset. Expected {_0:?}, got {_1:?}")]
    BlockMismatchEpochReset(BlockNumHash, BlockNumHash, B256),
    /// [SystemConfig] update failed.
    ///
    /// [SystemConfig]: op_alloy_genesis::SystemConfig
    #[display("System config update failed")]
    SystemConfigUpdate,
    /// Broken time invariant between L2 and L1.
    #[display("Time invariant broken. L1 origin: {_0:?} | Next L2 time: {_1} | L1 block: {_2:?} | L1 timestamp {_3:?}")]
    BrokenTimeInvariant(BlockNumHash, u64, BlockNumHash, u64),
    /// Attributes unavailable.
    #[display("Attributes unavailable")]
    AttributesUnavailable,
    /// A custom error.
    #[display("Error in attributes builder: {_0}")]
    Custom(String),
}

impl core::error::Error for BuilderError {}
