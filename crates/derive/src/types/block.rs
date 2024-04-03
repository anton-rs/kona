//! This module contains the various Block types.

use alloy_primitives::{BlockHash, BlockNumber, B256};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Block Header Info
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct BlockInfo {
    /// The block hash
    pub hash: B256,
    /// The block number
    pub number: u64,
    /// The parent block hash
    pub parent_hash: B256,
    /// The block timestamp
    pub timestamp: u64,
}

impl BlockInfo {
    /// Instantiates a new [BlockInfo].
    pub fn new(hash: B256, number: u64, parent_hash: B256, timestamp: u64) -> Self {
        Self { hash, number, parent_hash, timestamp }
    }

    /// Returns the block ID.
    pub fn id(&self) -> BlockID {
        BlockID { hash: self.hash, number: self.number }
    }
}

impl core::fmt::Display for BlockInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "BlockInfo {{ hash: {}, number: {}, parent_hash: {}, timestamp: {} }}",
            self.hash, self.number, self.parent_hash, self.timestamp
        )
    }
}

/// Block ID identifies a block by its hash and number
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BlockID {
    /// The block hash
    pub hash: BlockHash,
    /// The block number
    pub number: BlockNumber,
}

impl core::fmt::Display for BlockID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ hash: {}, number: {} }}", self.hash, self.number)
    }
}

/// An L2 Block Ref
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct L2BlockRef {
    /// The l2 block info
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub info: BlockInfo,
    /// The l1 origin of the l2 block
    #[cfg_attr(feature = "serde", serde(rename = "l1origin"))]
    pub l1_origin: BlockID,
    /// The distance to the first block of the associated epoch
    #[cfg_attr(feature = "serde", serde(rename = "sequenceNumber"))]
    pub sequence_number: u64,
}

/// L2 Block Header Info
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct L2BlockInfo {
    /// The base [BlockInfo]
    pub block_info: BlockInfo,
    /// The L1 origin [BlockId]
    pub l1_origin: BlockId,
    /// The sequence number of the L2 block
    pub seq_num: u64,
}

impl L2BlockInfo {
    /// Instantiates a new [L2BlockInfo].
    pub fn new(block_info: BlockInfo, l1_origin: BlockId, seq_num: u64) -> Self {
        Self { block_info, l1_origin, seq_num }
    }
}

/// A Block Identifier
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockId {
    /// The block hash
    Hash(BlockHash),
    /// The block number
    Number(BlockNumber),
    /// The block kind
    Kind(BlockKind),
}

impl Default for BlockId {
    fn default() -> Self {
        BlockId::Kind(BlockKind::Latest)
    }
}

/// The Block Kind
///
/// The block kinds are:
/// - `Earliest`: The earliest known block.
/// - `Latest`: The latest pending block.
/// - `Finalized`: The latest finalized block.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockKind {
    /// The earliest known block.
    Earliest,
    /// The latest pending block.
    Latest,
    /// The latest finalized block.
    Finalized,
}
