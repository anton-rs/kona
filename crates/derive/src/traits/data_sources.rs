//! Contains traits that describe the functionality of various data sources used in the derivation
//! pipeline's stages.

use crate::types::{BlockInfo, ExecutionPayloadEnvelope, L2BlockInfo, Receipt, StageResult};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::{Address, Bytes, B256};
use anyhow::Result;
use async_trait::async_trait;
use core::fmt::Debug;

/// Describes the functionality of a data source that can provide information from the blockchain.
#[async_trait]
pub trait ChainProvider {
    /// Returns the block at the given number, or an error if the block does not exist in the data
    /// source.
    async fn block_info_by_number(&self, number: u64) -> Result<BlockInfo>;

    /// Returns all receipts in the block with the given hash, or an error if the block does not
    /// exist in the data source.
    async fn receipts_by_hash(&self, hash: B256) -> Result<Vec<Receipt>>;
}

/// Describes the functionality of a data source that fetches safe blocks.
#[async_trait]
pub trait SafeBlockFetcher {
    /// Returns the L2 block info given a block number.
    /// Errors if the block does not exist.
    async fn l2_block_info_by_number(&self, number: u64) -> Result<L2BlockInfo>;

    /// Returns an execution payload for a given number.
    /// Errors if the execution payload does not exist.
    async fn payload_by_number(&self, number: u64) -> Result<ExecutionPayloadEnvelope>;
}

/// Describes the functionality of a data source that can provide data availability information.
#[async_trait]
pub trait DataAvailabilityProvider {
    /// An iterator over returned bytes data.
    type DataIter: DataIter<Bytes> + Send + Debug;

    /// Returns the data availability for the block with the given hash, or an error if the block
    /// does not exist in the data source.
    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter>;
}

/// Describes the behavior of a data iterator.
pub trait DataIter<T> {
    /// Returns the next item in the iterator, or [crate::types::StageError::Eof] if the iterator is
    /// exhausted.
    fn next(&mut self) -> StageResult<T>;
}
