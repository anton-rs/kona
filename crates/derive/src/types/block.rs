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
        Self {
            hash,
            number,
            parent_hash,
            timestamp,
        }
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

// impl TryFrom<BlockWithTransactions> for BlockInfo {
//     type Error = anyhow::Error;
//
//     fn try_from(block: BlockWithTransactions) -> anyhow::Result<Self> {
//         Ok(BlockInfo {
//             number: block.number.unwrap_or_default().to::<u64>(),
//             hash: block.hash.unwrap_or_default(),
//             parent_hash: block.parent_hash,
//             timestamp: block.timestamp.to::<u64>(),
//         })
//     }
// }

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

// /// A Block with Transactions
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
// #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
// pub struct Block {
//     /// Header of the block.
//     #[serde(flatten)]
//     pub header: Header,
//     /// Uncles' hashes.
//     pub uncles: Vec<B256>,
//     /// Block Transactions. In the case of an uncle block, this field is not included in RPC
//     /// responses, and when deserialized, it will be set to [BlockTransactions::Uncle].
//     #[serde(
//         skip_serializing_if = "BlockTransactions::is_uncle",
//         default = "BlockTransactions::uncle"
//     )]
//     pub transactions: BlockTransactions,
//     /// Integer the size of this block in bytes.
//     pub size: Option<U256>,
//     /// Withdrawals in the block.
//     #[serde(default, skip_serializing_if = "Option::is_none")]
//     pub withdrawals: Option<Vec<Withdrawal>>,
//     /// Support for arbitrary additional fields.
//     #[serde(flatten)]
//     pub other: OtherFields,
// }
