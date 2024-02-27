//! Contains traits that describe the functionality of various data sources used in the derivation pipeline's stages.

use crate::types::{BlockInfo, Receipt, StageResult, TxEnvelope};
use alloc::fmt::Debug;
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::{Address, Bytes, B256};
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

    /// Returns the [BlockInfo] and list of [TxEnvelope]s from the given block hash.
    async fn block_info_and_transactions_by_hash(
        &self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)>;
}

/// A simple asynchronous iterator trait.
/// This should be replaced with the `async-iterator` crate.
#[async_trait]
pub trait AsyncIterator {
    /// The item type of the iterator.
    type Item: Send + Sync + Debug;

    /// Returns the next item in the iterator, or [crate::types::StageError::Eof] if the iterator is exhausted.
    async fn next(&mut self) -> Option<Self::Item>;
}

/// Describes the functionality of a data source that can provide data availability information.
#[async_trait]
pub trait DataAvailabilityProvider {
    /// A data iterator for the data source to return.
    /// The iterator returns the next item in the iterator, or [crate::types::StageError::Eof] if the iterator is exhausted.
    type DataIter: AsyncIterator<Item = StageResult<Bytes>> + Send + Sync + Debug;

    /// Returns the data availability for the block with the given hash, or an error if the block does not exist in the
    /// data source.
    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter>;
}
