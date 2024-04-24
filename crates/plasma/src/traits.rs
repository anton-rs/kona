//! Traits for plasma sources and internal components.

use crate::types::{FinalizedHeadSignal, PlasmaError};
use alloc::{boxed::Box, vec::Vec};
use alloy_consensus::{Header, Receipt, TxEnvelope};
use alloy_primitives::{Bytes, B256};
use async_trait::async_trait;
use kona_primitives::{
    block::{BlockID, BlockInfo},
    system_config::SystemConfig,
};

/// Describes the functionality of a data source that can provide information from the blockchain.
#[async_trait]
pub trait ChainProvider {
    /// Fetch the L1 [Header] for the given [B256] hash.
    async fn header_by_hash(&mut self, hash: B256) -> anyhow::Result<Header>;

    /// Returns the block at the given number, or an error if the block does not exist in the data
    /// source.
    async fn block_info_by_number(&mut self, number: u64) -> anyhow::Result<BlockInfo>;

    /// Returns all receipts in the block with the given hash, or an error if the block does not
    /// exist in the data source.
    async fn receipts_by_hash(&mut self, hash: B256) -> anyhow::Result<Vec<Receipt>>;

    /// Returns the [BlockInfo] and list of [TxEnvelope]s from the given block hash.
    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> anyhow::Result<(BlockInfo, Vec<TxEnvelope>)>;
}

/// A plasma input fetcher.
#[async_trait]
pub trait PlasmaInputFetcher<CP: ChainProvider + Send> {
    /// Get the input for the given commitment at the given block number from the DA storage
    /// service.
    async fn get_input(
        &mut self,
        fetcher: &CP,
        commitment: Bytes,
        block: BlockID,
    ) -> Option<Result<Bytes, PlasmaError>>;

    /// Advance the L1 origin to the given block number, syncing the DA challenge events.
    async fn advance_l1_origin(
        &mut self,
        fetcher: &CP,
        block: BlockID,
    ) -> Option<Result<(), PlasmaError>>;

    /// Reset the challenge origin in case of L1 reorg.
    async fn reset(
        &mut self,
        block_number: BlockInfo,
        cfg: SystemConfig,
    ) -> Option<Result<(), PlasmaError>>;

    /// Notify L1 finalized head so plasma finality is always behind L1.
    async fn finalize(&mut self, block_number: BlockInfo) -> Option<Result<(), PlasmaError>>;

    /// Set the engine finalization signal callback.
    fn on_finalized_head_signal(&mut self, callback: FinalizedHeadSignal);
}
