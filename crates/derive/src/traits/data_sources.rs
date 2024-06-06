//! Contains traits that describe the functionality of various data sources used in the derivation
//! pipeline's stages.

use crate::types::{BlockInfo, StageResult};
use alloc::{boxed::Box, fmt::Debug};
use alloy_primitives::{Address, Bytes};
use anyhow::Result;
use async_trait::async_trait;

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
