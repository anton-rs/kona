//! Contains traits that describe the functionality of various data sources used in the derivation pipeline's stages.

// use alloy_rpc_types::Block;
use crate::types::{BlockInfo, Receipt};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::B256;
use anyhow::Result;
use async_trait::async_trait;

/// Describes the functionality of a data source that can provide information from the blockchain.
#[async_trait]
pub trait ChainProvider {
    /// Returns the block at the given number, or an error if the block does not exist in the data source.
    async fn block_info_by_number(&self, number: u64) -> Result<BlockInfo>;

    /// Returns all receipts in the block with the given hash, or an error if the block does not exist in the data
    /// source.
    async fn receipts_by_hash(&self, hash: B256) -> Result<Vec<Receipt>>;
}
