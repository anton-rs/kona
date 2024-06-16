//! This module contains the [HintType] enum.

use alloc::{string::String, vec::Vec};
use alloy_primitives::hex;
use core::fmt::Display;

/// The [HintType] enum is used to specify the type of hint that was received.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HintType {
    /// A hint that specifies the block header of a layer 1 block.
    L1BlockHeader,
    /// A hint that specifies the transactions of a layer 1 block.
    L1Transactions,
    /// A hint that specifies the state node of a layer 1 block.
    L1Receipts,
    /// A hint that specifies a blob in the layer 1 beacon chain.
    L1Blob,
    /// A hint that specifies a precompile call on layer 1.
    L1Precompile,
    /// A hint that specifies the block header of a layer 2 block.
    L2BlockHeader,
    /// A hint that specifies the transactions of a layer 2 block.
    L2Transactions,
    /// A hint that specifies the code of a contract on layer 2.
    L2Code,
    /// A hint that specifies the preimage of the starting L2 output root on layer 2.
    StartingL2Output,
    /// A hint that specifies the state node in the L2 state trie.
    L2StateNode,
    /// A hint that specifies the proof on the path to an account in the L2 state trie.
    L2AccountProof,
    /// A hint that specifies the proof on the path to a storage slot in an account within in the
    /// L2 state trie.
    L2AccountStorageProof,
}

impl HintType {
    /// Encodes the hint type as a string.
    pub fn encode_with(&self, data: &[&[u8]]) -> String {
        let concatenated = hex::encode(data.iter().copied().flatten().copied().collect::<Vec<_>>());
        alloc::format!("{} {}", self, concatenated)
    }
}

impl TryFrom<&str> for HintType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "l1-block-header" => Ok(HintType::L1BlockHeader),
            "l1-transactions" => Ok(HintType::L1Transactions),
            "l1-receipts" => Ok(HintType::L1Receipts),
            "l1-blob" => Ok(HintType::L1Blob),
            "l1-precompile" => Ok(HintType::L1Precompile),
            "l2-block-header" => Ok(HintType::L2BlockHeader),
            "l2-transactions" => Ok(HintType::L2Transactions),
            "l2-code" => Ok(HintType::L2Code),
            "starting-l2-output" => Ok(HintType::StartingL2Output),
            "l2-state-node" => Ok(HintType::L2StateNode),
            "l2-account-proof" => Ok(HintType::L2AccountProof),
            "l2-account-storage-proof" => Ok(HintType::L2AccountStorageProof),
            _ => anyhow::bail!("Invalid hint type: {value}"),
        }
    }
}

impl From<HintType> for &str {
    fn from(value: HintType) -> Self {
        match value {
            HintType::L1BlockHeader => "l1-block-header",
            HintType::L1Transactions => "l1-transactions",
            HintType::L1Receipts => "l1-receipts",
            HintType::L1Blob => "l1-blob",
            HintType::L1Precompile => "l1-precompile",
            HintType::L2BlockHeader => "l2-block-header",
            HintType::L2Transactions => "l2-transactions",
            HintType::L2Code => "l2-code",
            HintType::StartingL2Output => "starting-l2-output",
            HintType::L2StateNode => "l2-state-node",
            HintType::L2AccountProof => "l2-account-proof",
            HintType::L2AccountStorageProof => "l2-account-storage-proof",
        }
    }
}

impl Display for HintType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s: &str = (*self).into();
        write!(f, "{}", s)
    }
}
