use core::fmt::Display;

use alloc::{boxed::Box, string::ToString, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Receipt, TxEnvelope};
use alloy_primitives::B256;
use async_trait::async_trait;
use kona_primitives::L2ExecutionPayloadEnvelope;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BlockInfo, L2BlockInfo};

/// Describes the functionality of a data source that can provide information from the blockchain.
#[async_trait]
pub trait ChainProvider {
    /// The error type for the [ChainProvider].
    type Error: Display + ToString;

    /// Fetch the L1 [Header] for the given [B256] hash.
    async fn header_by_hash(&mut self, hash: B256) -> Result<Header, Self::Error>;

    /// Returns the block at the given number, or an error if the block does not exist in the data
    /// source.
    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo, Self::Error>;

    /// Returns all receipts in the block with the given hash, or an error if the block does not
    /// exist in the data source.
    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>, Self::Error>;

    /// Returns the [BlockInfo] and list of [TxEnvelope]s from the given block hash.
    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>), Self::Error>;
}

/// Describes the functionality of a data source that fetches safe blocks.
#[async_trait]
pub trait L2ChainProvider {
    /// The error type for the [L2ChainProvider].
    type Error: Display + ToString;

    /// Returns the L2 block info given a block number.
    /// Errors if the block does not exist.
    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo, Self::Error>;

    /// Returns an execution payload for a given number.
    /// Errors if the execution payload does not exist.
    async fn payload_by_number(
        &mut self,
        number: u64,
    ) -> Result<L2ExecutionPayloadEnvelope, Self::Error>;

    /// Returns the [SystemConfig] by L2 number.
    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig, Self::Error>;
}
