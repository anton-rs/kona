//! This module defines the [ChainProvider] trait.

use alloc::vec::Vec;
use anyhow::Result;
use async_trait::async_trait;
use alloy_primitives::B256;
use kona_primitives::block::BlockInfo;
use alloy_consensus::{Header, Receipt, TxEnvelope};

/// Describes the functionality of a data source that can provide information from the blockchain.
#[async_trait]
pub trait ChainProvider {
    /// Fetch the L1 [Header] for the given [B256] hash.
    async fn header_by_hash(&mut self, hash: B256) -> Result<Header>;

    /// Returns the block at the given number, or an error if the block does not exist in the data
    /// source.
    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo>;

    /// Returns all receipts in the block with the given hash, or an error if the block does not
    /// exist in the data source.
    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>>;

    /// Returns the [BlockInfo] and list of [TxEnvelope]s from the given block hash.
    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)>;
}
