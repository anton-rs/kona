//! This module defines the [L2ChainProvider] trait.

use alloc::{boxed::Box, sync::Arc};
use anyhow::Result;
use async_trait::async_trait;
use kona_primitives::{
    block::L2BlockInfo, payload::L2ExecutionPayloadEnvelope, rollup_config::RollupConfig,
    system_config::SystemConfig,
};

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
