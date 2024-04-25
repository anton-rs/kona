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

/// An [op_alloy_consensus::TxDeposit] validation error.
#[derive(Debug)]
pub enum DepositError {
    /// Unexpected number of deposit event log topics.
    UnexpectedTopicsLen(usize),
    /// Invalid deposit event selector.
    /// Expected: [B256] (deposit event selector), Actual: [B256] (event log topic).
    InvalidSelector(B256, B256),
    /// Incomplete opaqueData slice header (incomplete length).
    IncompleteOpaqueData(usize),
    /// The log data is not aligned to 32 bytes.
    UnalignedData(usize),
    /// Failed to decode the `from` field of the deposit event (the second topic).
    FromDecode(B256),
    /// Failed to decode the `to` field of the deposit event (the third topic).
    ToDecode(B256),
    /// Invalid opaque data content offset.
    InvalidOpaqueDataOffset(Bytes),
    /// Invalid opaque data content length.
    InvalidOpaqueDataLength(Bytes),
    /// Opaque data length exceeds the deposit log event data length.
    /// Specified: [usize] (data length), Actual: [usize] (opaque data length).
    OpaqueDataOverflow(usize, usize),
    /// Opaque data with padding exceeds the specified data length.
    PaddedOpaqueDataOverflow(usize, usize),
    /// An invalid deposit version.
    InvalidVersion(B256),
    /// Unexpected opaque data length
    UnexpectedOpaqueDataLen(usize),
    /// Failed to decode the deposit mint value.
    MintDecode(Bytes),
    /// Failed to decode the deposit gas value.
    GasDecode(Bytes),
    /// A custom error wrapping [anyhow::Error].
    Custom(anyhow::Error),
}

impl PartialEq<DepositError> for DepositError {
    fn eq(&self, other: &DepositError) -> bool {
        match (self, other) {
            (DepositError::UnexpectedTopicsLen(l1), DepositError::UnexpectedTopicsLen(l2)) => {
                l1 == l2
            }
            (DepositError::InvalidSelector(e1, t1), DepositError::InvalidSelector(e2, t2)) => {
                e1 == e2 && t1 == t2
            }
            (DepositError::IncompleteOpaqueData(l1), DepositError::IncompleteOpaqueData(l2)) => {
                l1 == l2
            }
            (DepositError::UnalignedData(d1), DepositError::UnalignedData(d2)) => d1 == d2,
            (DepositError::FromDecode(e1), DepositError::FromDecode(e2)) => e1 == e2,
            (DepositError::ToDecode(e1), DepositError::ToDecode(e2)) => e1 == e2,
            (
                DepositError::InvalidOpaqueDataOffset(o1),
                DepositError::InvalidOpaqueDataOffset(o2),
            ) => o1 == o2,
            (
                DepositError::InvalidOpaqueDataLength(o1),
                DepositError::InvalidOpaqueDataLength(o2),
            ) => o1 == o2,
            (
                DepositError::OpaqueDataOverflow(l1, l2),
                DepositError::OpaqueDataOverflow(l3, l4),
            ) => l1 == l3 && l2 == l4,
            (
                DepositError::PaddedOpaqueDataOverflow(l1, l2),
                DepositError::PaddedOpaqueDataOverflow(l3, l4),
            ) => l1 == l3 && l2 == l4,
            (DepositError::InvalidVersion(v1), DepositError::InvalidVersion(v2)) => v1 == v2,
            (
                DepositError::UnexpectedOpaqueDataLen(a),
                DepositError::UnexpectedOpaqueDataLen(b),
            ) => a == b,
            (DepositError::MintDecode(a), DepositError::MintDecode(b)) => a == b,
            (DepositError::GasDecode(a), DepositError::GasDecode(b)) => a == b,
            (DepositError::Custom(_), DepositError::Custom(_)) => true,
            _ => false,
        }
    }
}

impl Display for DepositError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DepositError::UnexpectedTopicsLen(len) => {
                write!(f, "Unexpected number of deposit event log topics: {}", len)
            }
            DepositError::InvalidSelector(expected, actual) => {
                write!(f, "Invalid deposit event selector: {}, expected {}", actual, expected)
            }
            DepositError::IncompleteOpaqueData(len) => {
                write!(f, "Incomplete opaqueData slice header (incomplete length): {}", len)
            }
            DepositError::UnalignedData(data) => {
                write!(f, "Unaligned log data, expected multiple of 32 bytes, got: {}", data)
            }
            DepositError::FromDecode(topic) => {
                write!(f, "Failed to decode the `from` address of the deposit log topic: {}", topic)
            }
            DepositError::ToDecode(topic) => {
                write!(f, "Failed to decode the `to` address of the deposit log topic: {}", topic)
            }
            DepositError::InvalidOpaqueDataOffset(offset) => {
                write!(f, "Invalid u64 opaque data content offset: {:?}", offset)
            }
            DepositError::InvalidOpaqueDataLength(length) => {
                write!(f, "Invalid u64 opaque data content length: {:?}", length)
            }
            DepositError::OpaqueDataOverflow(data_len, opaque_len) => {
                write!(
                    f,
                    "Specified opaque data length {} exceeds the deposit log event data length {}",
                    opaque_len, data_len
                )
            }
            DepositError::PaddedOpaqueDataOverflow(data_len, opaque_len) => {
                write!(
                    f,
                    "Opaque data with padding exceeds the specified data length: {} > {}",
                    opaque_len, data_len
                )
            }
            DepositError::InvalidVersion(version) => {
                write!(f, "Invalid deposit version: {}", version)
            }
            DepositError::UnexpectedOpaqueDataLen(len) => {
                write!(f, "Unexpected opaque data length: {}", len)
            }
            DepositError::MintDecode(data) => {
                write!(f, "Failed to decode the u128 deposit mint value: {:?}", data)
            }
            DepositError::GasDecode(data) => {
                write!(f, "Failed to decode the u64 deposit gas value: {:?}", data)
            }
            DepositError::Custom(e) => write!(f, "Custom error: {}", e),
        }
    }
}
