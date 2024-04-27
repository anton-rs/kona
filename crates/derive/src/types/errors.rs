//! This module contains derivation errors thrown within the pipeline.

use super::SpanBatchError;
use crate::types::{BlockID, Frame};
use alloc::vec::Vec;
use alloy_primitives::{Bytes, B256};
use core::fmt::Display;
use kona_plasma::types::PlasmaError;

/// A result type for the derivation pipeline stages.
pub type StageResult<T> = Result<T, StageError>;

/// An error that is thrown within the stages of the derivation pipeline.
#[derive(Debug)]
pub enum StageError {
    /// There is no data to read from the channel bank.
    Eof,
    /// A temporary error that allows the operation to be retried.
    Temporary(anyhow::Error),
    /// A critical error.
    Critical(anyhow::Error),
    /// Plasma data source error.
    Plasma(PlasmaError),
    /// There is not enough data progress, but if we wait, the stage will eventually return data
    /// or produce an EOF error.
    NotEnoughData,
    /// Failed to fetch block info and transactions by hash.
    BlockFetch(B256),
    /// No item returned from the previous stage iterator.
    Empty,
    /// No channels are available in the channel bank.
    NoChannelsAvailable,
    /// No channel returned by the [crate::stages::ChannelReader] stage.
    NoChannel,
    /// Failed to find channel.
    ChannelNotFound,
    /// Missing L1 origin.
    MissingOrigin,
    /// Failed to build the [L2PayloadAttributes] for the next batch.
    ///
    /// [L2PayloadAttributes]: super::L2PayloadAttributes
    AttributesBuild(BuilderError),
    /// Reset the pipeline.
    Reset(ResetError),
    /// The stage detected a block reorg.
    /// The first argument is the expected block hash.
    /// The second argument is the paren_hash of the next l1 origin block.
    ReorgDetected(B256, B256),
    /// Receipt fetching error.
    ReceiptFetch(anyhow::Error),
    /// [super::BlockInfo] fetching error.
    BlockInfoFetch(anyhow::Error),
    /// [super::SystemConfig] update error.
    SystemConfigUpdate(anyhow::Error),
    /// Other wildcard error.
    Custom(anyhow::Error),
}

impl PartialEq<StageError> for StageError {
    fn eq(&self, other: &StageError) -> bool {
        // if it's a reorg detected check the block hashes
        if let (StageError::ReorgDetected(a, b), StageError::ReorgDetected(c, d)) = (self, other) {
            return a == c && b == d;
        }
        if let (StageError::Reset(a), StageError::Reset(b)) = (self, other) {
            return a == b;
        }
        matches!(
            (self, other),
            (StageError::Eof, StageError::Eof) |
                (StageError::Temporary(_), StageError::Temporary(_)) |
                (StageError::Critical(_), StageError::Critical(_)) |
                (StageError::Plasma(_), StageError::Plasma(_)) |
                (StageError::NotEnoughData, StageError::NotEnoughData) |
                (StageError::NoChannelsAvailable, StageError::NoChannelsAvailable) |
                (StageError::NoChannel, StageError::NoChannel) |
                (StageError::ChannelNotFound, StageError::ChannelNotFound) |
                (StageError::MissingOrigin, StageError::MissingOrigin) |
                (StageError::AttributesBuild(_), StageError::AttributesBuild(_)) |
                (StageError::ReceiptFetch(_), StageError::ReceiptFetch(_)) |
                (StageError::BlockInfoFetch(_), StageError::BlockInfoFetch(_)) |
                (StageError::SystemConfigUpdate(_), StageError::SystemConfigUpdate(_)) |
                (StageError::Custom(_), StageError::Custom(_))
        )
    }
}

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
            StageError::Temporary(e) => write!(f, "Temporary error: {}", e),
            StageError::Critical(e) => write!(f, "Critical error: {}", e),
            StageError::Plasma(e) => write!(f, "Plasma error: {:?}", e),
            StageError::NotEnoughData => write!(f, "Not enough data"),
            StageError::BlockFetch(hash) => {
                write!(f, "Failed to fetch block info and transactions by hash: {}", hash)
            }
            StageError::Empty => write!(f, "Empty"),
            StageError::NoChannelsAvailable => write!(f, "No channels available"),
            StageError::NoChannel => write!(f, "No channel"),
            StageError::ChannelNotFound => write!(f, "Channel not found"),
            StageError::MissingOrigin => write!(f, "Missing L1 origin"),
            StageError::AttributesBuild(e) => write!(f, "Attributes build error: {}", e),
            StageError::Reset(e) => write!(f, "Reset error: {}", e),
            StageError::ReceiptFetch(e) => write!(f, "Receipt fetch error: {}", e),
            StageError::SystemConfigUpdate(e) => write!(f, "System config update error: {}", e),
            StageError::ReorgDetected(current, next) => {
                write!(f, "Block reorg detected: {} -> {}", current, next)
            }
            StageError::BlockInfoFetch(e) => write!(f, "Block info fetch error: {}", e),
            StageError::Custom(e) => write!(f, "Custom error: {}", e),
        }
    }
}

/// A reset error
#[derive(Debug)]
pub enum ResetError {
    /// The batch has a bad parent hash.
    /// The first argument is the expected parent hash, and the second argument is the actual
    /// parent hash.
    BadParentHash(B256, B256),
    /// The batch has a bad timestamp.
    /// The first argument is the expected timestamp, and the second argument is the actual
    /// timestamp.
    BadTimestamp(u64, u64),
    /// A reorg is required.
    ReorgRequired,
    /// A new expired challenge.
    NewExpiredChallenge,
}

impl PartialEq<ResetError> for ResetError {
    fn eq(&self, other: &ResetError) -> bool {
        match (self, other) {
            (ResetError::BadParentHash(e1, a1), ResetError::BadParentHash(e2, a2)) => {
                e1 == e2 && a1 == a2
            }
            (ResetError::BadTimestamp(e1, a1), ResetError::BadTimestamp(e2, a2)) => {
                e1 == e2 && a1 == a2
            }
            (ResetError::ReorgRequired, ResetError::ReorgRequired) => true,
            (ResetError::NewExpiredChallenge, ResetError::NewExpiredChallenge) => true,
            _ => false,
        }
    }
}

impl Display for ResetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ResetError::BadParentHash(expected, actual) => {
                write!(f, "Bad parent hash: expected {}, got {}", expected, actual)
            }
            ResetError::BadTimestamp(expected, actual) => {
                write!(f, "Bad timestamp: expected {}, got {}", expected, actual)
            }
            ResetError::ReorgRequired => write!(f, "Reorg required"),
            ResetError::NewExpiredChallenge => write!(f, "New expired challenge"),
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
    /// Span Batch Error.
    SpanBatchError(SpanBatchError),
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
            (DecodeError::EmptyBuffer, DecodeError::EmptyBuffer) |
                (DecodeError::AlloyRlpError(_), DecodeError::AlloyRlpError(_))
        )
    }
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::EmptyBuffer => write!(f, "Empty buffer"),
            DecodeError::AlloyRlpError(e) => write!(f, "Alloy RLP Decoding Error: {}", e),
            DecodeError::SpanBatchError(e) => write!(f, "Span Batch Decoding Error: {:?}", e),
        }
    }
}

/// An [AttributesBuilder] Error.
///
/// [AttributesBuilder]: crate::stages::AttributesBuilder
#[derive(Debug)]
pub enum BuilderError {
    /// Mismatched blocks.
    BlockMismatch(BlockID, BlockID),
    /// Mismatched blocks for the start of an Epoch.
    BlockMismatchEpochReset(BlockID, BlockID, B256),
    /// [SystemConfig] update failed.
    ///
    /// [SystemConfig]: crate::types::SystemConfig
    SystemConfigUpdate,
    /// Broken time invariant between L2 and L1.
    BrokenTimeInvariant(BlockID, u64, BlockID, u64),
    /// A custom error wrapping [anyhow::Error].
    Custom(anyhow::Error),
}

impl PartialEq<BuilderError> for BuilderError {
    fn eq(&self, other: &BuilderError) -> bool {
        match (self, other) {
            (BuilderError::BlockMismatch(b1, e1), BuilderError::BlockMismatch(b2, e2)) => {
                b1 == b2 && e1 == e2
            }
            (
                BuilderError::BlockMismatchEpochReset(b1, e1, e2),
                BuilderError::BlockMismatchEpochReset(b2, e3, e4),
            ) => e1 == e3 && e2 == e4 && b1 == b2,
            (
                BuilderError::BrokenTimeInvariant(b1, t1, b2, t2),
                BuilderError::BrokenTimeInvariant(b3, t3, b4, t4),
            ) => b1 == b3 && t1 == t3 && b2 == b4 && t2 == t4,
            (BuilderError::SystemConfigUpdate, BuilderError::SystemConfigUpdate) |
            (BuilderError::Custom(_), BuilderError::Custom(_)) => true,
            _ => false,
        }
    }
}

impl From<anyhow::Error> for BuilderError {
    fn from(e: anyhow::Error) -> Self {
        BuilderError::Custom(e)
    }
}

impl Display for BuilderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BuilderError::BlockMismatch(block_id, parent) => {
                write!(f, "Block mismatch with L1 origin {} (parent {})", block_id, parent)
            }
            BuilderError::BlockMismatchEpochReset(block_id, parent, origin) => {
                write!(
                    f,
                    "Block mismatch with L1 origin {} (parent {}) on top of L1 origin {}",
                    block_id, parent, origin
                )
            }
            BuilderError::SystemConfigUpdate => write!(f, "System config update failed"),
            BuilderError::BrokenTimeInvariant(block_id, l2_time, parent, l1_time) => {
                write!(
                    f,
                    "Cannot build L2 block on top {} (time {}) before L1 origin {} (time {})",
                    block_id, l2_time, parent, l1_time
                )
            }
            BuilderError::Custom(e) => write!(f, "Custom error: {}", e),
        }
    }
}
