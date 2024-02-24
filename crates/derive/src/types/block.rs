//! This module contains the various Block types.
//!
//! TODO: Update for Cancun.

use crate::types::Transaction;
use alloc::vec::Vec;
use alloy_primitives::{Address, BlockHash, BlockNumber, Bloom, Bytes, B256, U256, U64};

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

impl TryFrom<BlockWithTransactions> for BlockInfo {
    type Error = anyhow::Error;

    fn try_from(block: BlockWithTransactions) -> anyhow::Result<Self> {
        Ok(BlockInfo {
            number: block.number.unwrap_or_default().to::<u64>(),
            hash: block.hash.unwrap_or_default(),
            parent_hash: block.parent_hash,
            timestamp: block.timestamp.to::<u64>(),
        })
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

/// A Block with Transactions
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockWithTransactions {
    /// The block hash
    pub hash: Option<B256>,
    /// The parent block hash
    pub parent_hash: B256,
    /// The block author
    pub author: Option<Address>,
    /// The block state root hash
    pub state_root: B256,
    /// The block transactions root hash
    pub transactions_root: B256,
    /// The block receipts root hash
    pub receipts_root: B256,
    /// The block number
    pub number: Option<U64>,
    /// The amount of gas used in the block
    pub gas_used: U256,
    /// Block extra data
    pub extra_data: Bytes,
    /// The block logs bloom filter
    pub logs_bloom: Option<Bloom>,
    /// The block timestamp
    pub timestamp: U256,
    /// The block total difficulty
    pub total_difficulty: Option<U256>,
    /// The block seal fields
    pub seal_fields: Vec<Bytes>,
    /// The block transactions
    pub transactions: Vec<Transaction>,
    /// The block size
    pub size: Option<U256>,
    /// The block base fee per gas
    pub base_fee_per_gas: Option<U256>,
}
