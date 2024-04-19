//! Contains traits that describe the functionality of various data sources used in the derivation
//! pipeline's stages.

use crate::types::{
    Blob, BlockInfo, IndexedBlobHash, L2BlockInfo, L2ExecutionPayloadEnvelope, RollupConfig,
    StageResult, SystemConfig,
};
use alloc::{boxed::Box, fmt::Debug, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Receipt, TxEnvelope};
use alloy_primitives::{Address, Bytes, B256};
use anyhow::Result;
use async_trait::async_trait;

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

/// Describes the functionality of a data source that fetches safe blocks.
#[async_trait]
pub trait L2ChainProvider {
    /// Returns the L2 block info given a block number.
    /// Errors if the block does not exist.
    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo>;

    /// Returns an execution payload for a given number.
    /// Errors if the execution payload does not exist.
    async fn payload_by_number(&mut self, number: u64) -> Result<L2ExecutionPayloadEnvelope>;

    /// Returns the [SystemConfig] by L2 number.
    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig>;
}

/// The BlobProvider trait specifies the functionality of a data source that can provide blobs.
#[async_trait]
pub trait BlobProvider {
    /// Fetches blobs for a given block ref and the blob hashes.
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: Vec<IndexedBlobHash>,
    ) -> Result<Vec<Blob>>;
}

/// The PlasmaProvider trait specifies the functionality of a data source that can fetch plasma
/// inputs.
#[async_trait]
#[allow(dead_code)]
pub(crate) trait PlasmaProvider {
    /// Fetches the plasma input for the given commitment at the given block number.
    async fn get_input(&self, commitment: &[u8], block_number: u64) -> Result<Bytes>;
}

/// Describes the functionality of a data source that can provide data availability information.
#[async_trait]
pub trait DataAvailabilityProvider {
    /// The item type of the data iterator.
    type Item: Send + Sync + Debug + Into<Bytes>;
    /// An iterator over returned bytes data.
    type DataIter: AsyncIterator<Item = Self::Item> + Send + Debug;

    /// Returns the data availability for the block with the given hash, or an error if the block
    /// does not exist in the data source.
    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter>;
}

/// A simple asynchronous iterator trait.
/// This should be replaced with the `async-iterator` crate
#[async_trait]
pub trait AsyncIterator {
    /// The item type of the iterator.
    type Item: Send + Sync + Debug + Into<Bytes>;

    /// Returns the next item in the iterator, or [crate::types::StageError::Eof] if the iterator is
    /// exhausted.
    async fn next(&mut self) -> Option<StageResult<Self::Item>>;
}
