//! L2 Chain Provider

use alloc::{boxed::Box, string::ToString, sync::Arc};
use async_trait::async_trait;
use core::fmt::Display;
use op_alloy_consensus::OpBlock;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::L2BlockInfo;

/// Describes the functionality of a data source that fetches safe blocks.
#[async_trait]
pub trait L2ChainProvider {
    /// The error type for the [L2ChainProvider].
    type Error: Display + ToString;

    /// Returns the L2 block info given a block number.
    /// Errors if the block does not exist.
    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo, Self::Error>;

    /// Returns the block for a given number.
    /// Errors if no block is available for the given block number.
    async fn block_by_number(&mut self, number: u64) -> Result<OpBlock, Self::Error>;

    /// Returns the [SystemConfig] by L2 number.
    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig, Self::Error>;
}
