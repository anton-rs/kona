//! Contains traits that describe the functionality of various data sources used in the derivation pipeline's stages.

// use alloy_rpc_types::Block;
use alloc::boxed::Box;
use anyhow::Result;
use async_trait::async_trait;

/// Describes the functionality of a data source that can provide a block by number.
#[async_trait]
pub trait BlockByNumberProvider {
    /// Returns the block at the given number, or an error if the block does not exist in the data source.
    async fn block_by_number(&self, number: u64) -> Result<()>;
}
