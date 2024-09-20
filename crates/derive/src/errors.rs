//! This module contains derivation errors thrown within the pipeline.

use crate::batch::SpanBatchError;
use alloy_eips::BlockNumHash;
use alloy_primitives::B256;
use kona_primitives::BlobDecodingError;
use op_alloy_genesis::system::SystemConfigUpdateError;
use op_alloy_protocol::DepositError;
use thiserror::Error;
use alloc::string::String;

/// A result type for the derivation pipeline stages.
pub type PipelineResult<T> = Result<T, StageErrorKind>;

/// [ensure] is a short-hand for bubbling up errors in the case of a condition not being met.
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err);
        }
    };
}

/// A top level filter for [StageError] that sorts by severity.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum StageErrorKind {
    /// A temporary error.
    #[error("Temporary error: {0}")]
    Temporary(#[source] PipelineError),
    /// A critical error.
    #[error("Critical error: {0}")]
    Critical(#[source] PipelineError),
    /// A reset error.
    #[error("Pipeline reset: {0}")]
    Reset(#[from] ResetError),
}

/// An error encountered during the processing.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum PipelineError {
    /// There is no data to read from the channel bank.
    #[error("EOF")]
    Eof,
    /// There is not enough data to complete the processing of the stage. If the operation is re-tried,
    /// more data will come in allowing the pipeline to progress, or eventually a [StageErrorNew::Eof]
    /// will be encountered.
    #[error("Not enough data")]
    NotEnoughData,
    /// No channels are available in the [ChannelBank].
    ///
    /// [ChannelBank]: crate::channel::ChannelBank
    #[error("The channel bank is empty")]
    ChannelBankEmpty,
    /// No channel returned by the [ChannelReader] stage.
    ///
    /// [ChannelReader]: crate::stages::ChannelReader
    #[error("The channel reader has no channel available")]
    ChannelReaderEmpty,
    /// The [BatchQueue] is empty.
    ///
    /// [BatchQueue]: crate::statges::BatchQueue
    #[error("The batch queue has no batches available")]
    BatchQueueEmpty,
    /// Failed to find channel in the [ChannelBank].
    ///
    /// [ChannelBank]: crate::channel::ChannelBank
    #[error("Channel not found in channel bank")]
    ChannelNotFound,
    /// Missing L1 origin.
    #[error("Missing L1 origin from previous stage")]
    MissingOrigin,
    /// Missing data from [L1Retrieval].
    #[error("L1 Retrieval missing data")]
    MissingL1Data,
    /// [SystemConfig] update error.
    ///
    /// [SystemConfig]: op_alloy_genesis::SystemConfig
    #[error("Error updating system config: {0}")]
    SystemConfigUpdate(SystemConfigUpdateError),
    /// Attributes builder error variant, with [BuilderError].
    #[error("Attributes builder error: {0}")]
    AttributesBuilder(#[from] BuilderError),
    /// [DecodeError] variant.
    #[error("Decode error: {0}")]
    DecodeError(#[from] DecodeError),
    /// Provider error variant.
    #[error("Blob provider error: {0}")]
    Provider(String),
    /// Custom error variant.
    #[error("Pipeline error: {0}")]
    Custom(String),
}

impl PipelineError {
    /// Wrap [self] as a [StageErrorKind::Critical].
    pub fn crit(self) -> StageErrorKind {
        StageErrorKind::Critical(self)
    }

    /// Wrap [self] as a [StageErrorKind::Temporary].
    pub fn temp(self) -> StageErrorKind {
        StageErrorKind::Temporary(self)
    }
}

/// A reset error
#[derive(Error, Debug, Eq, PartialEq)]
pub enum ResetError {
    /// The batch has a bad parent hash.
    /// The first argument is the expected parent hash, and the second argument is the actual
    /// parent hash.
    #[error("Bad parent hash: expected {0}, got {1}")]
    BadParentHash(B256, B256),
    /// The batch has a bad timestamp.
    /// The first argument is the expected timestamp, and the second argument is the actual
    /// timestamp.
    #[error("Bad timestamp: expected {0}, got {1}")]
    BadTimestamp(u64, u64),
    /// L1 origin mismatch.
    #[error("L1 origin mismatch. Expected {0:?}, got {1:?}")]
    L1OriginMismatch(u64, u64),
    /// The stage detected a block reorg.
    /// The first argument is the expected block hash.
    /// The second argument is the parent_hash of the next l1 origin block.
    #[error("L1 reorg detected: expected {0}, got {1}")]
    ReorgDetected(B256, B256),
    /// Attributes builder error variant, with [BuilderError].
    #[error("Attributes builder error: {0}")]
    AttributesBuilder(#[from] BuilderError),
}

/// An error returned by the [BlobProviderError].
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BlobProviderError {
    /// The number of specified blob hashes did not match the number of returned sidecars.
    #[error("Blob sidecar length mismatch: expected {0}, got {1}")]
    SidecarLengthMismatch(usize, usize),
    /// Slot derivation error.
    #[error("Failed to derive slot")]
    SlotDerivation,
    /// Blob decoding error.
    #[error("Blob decoding error: {0}")]
    BlobDecoding(#[from] BlobDecodingError),
    /// Error pertaining to the backend transport.
    #[error("Blob provider backend error: {0}")]
    Backend(String),
}

/// A decoding error.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The buffer is empty.
    #[error("Empty buffer")]
    EmptyBuffer,
    /// Deposit decoding error.
    #[error("Error decoding deposit: {0}")]
    DepositError(#[from] DepositError),
    /// Alloy RLP Encoding Error.
    #[error(transparent)]
    AlloyRlpError(#[from] alloy_rlp::Error),
    /// Span Batch Error.
    #[error(transparent)]
    SpanBatchError(#[from] SpanBatchError),
}

/// A frame decompression error.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BatchDecompressionError {
    /// The buffer exceeds the [FJORD_MAX_SPAN_BATCH_BYTES] protocol parameter.
    ///
    /// [FJORD_MAX_SPAN_BATCH_BYTES]: crate::batch::FJORD_MAX_SPAN_BATCH_BYTES
    #[error("The batch exceeds the maximum size of {max_size} bytes", max_size = crate::batch::FJORD_MAX_SPAN_BATCH_BYTES)]
    BatchTooLarge,
}

/// An [AttributesBuilder] Error.
///
/// [AttributesBuilder]: crate::stages::AttributesBuilder
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BuilderError {
    /// Mismatched blocks.
    #[error("Block mismatch. Expected {0:?}, got {1:?}")]
    BlockMismatch(BlockNumHash, BlockNumHash),
    /// Mismatched blocks for the start of an Epoch.
    #[error("Block mismatch on epoch reset. Expected {0:?}, got {1:?}")]
    BlockMismatchEpochReset(BlockNumHash, BlockNumHash, B256),
    /// [SystemConfig] update failed.
    ///
    /// [SystemConfig]: op_alloy_genesis::SystemConfig
    #[error("System config update failed")]
    SystemConfigUpdate,
    /// Broken time invariant between L2 and L1.
    #[error("Time invariant broken. L1 origin: {0:?} | Next L2 time: {1} | L1 block: {2:?} | L1 timestamp {3:?}")]
    BrokenTimeInvariant(BlockNumHash, u64, BlockNumHash, u64),
    /// Attributes unavailable.
    #[error("Attributes unavailable")]
    AttributesUnavailable,
    /// A custom error.
    #[error("Error in attributes builder: {0}")]
    Custom(String),
}
