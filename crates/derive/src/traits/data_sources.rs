//! Contains traits that describe the functionality of various data sources used in the derivation
//! pipeline's stages.

use crate::errors::StageResult;
use alloc::{boxed::Box, fmt::Debug, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Receipt, TxEnvelope};
use alloy_primitives::{Address, Bytes, B256};
use anyhow::Result;
use async_trait::async_trait;
use kona_primitives::block::BlockInfo;
use kona_primitives::blob::{Blob, IndexedBlobHash};
pub use kona_providers::prelude::{ChainProvider, L2ChainProvider};

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
