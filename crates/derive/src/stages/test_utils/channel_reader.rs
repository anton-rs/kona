//! Test utilities for the [ChannelReader] stage.

use crate::{
    errors::{StageError, StageResult},
    stages::ChannelReaderProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_primitives::{BlockInfo, SystemConfig};

/// A mock [ChannelReaderProvider] for testing the [ChannelReader] stage.
#[derive(Debug, Default)]
pub struct MockChannelReaderProvider {
    /// The data to return.
    pub data: Vec<StageResult<Option<Bytes>>>,
    /// The origin block info
    pub block_info: Option<BlockInfo>,
}

impl MockChannelReaderProvider {
    /// Creates a new [MockChannelReaderProvider] with the given data.
    pub fn new(data: Vec<StageResult<Option<Bytes>>>) -> Self {
        Self { data, block_info: Some(BlockInfo::default()) }
    }
}

impl OriginProvider for MockChannelReaderProvider {
    fn origin(&self) -> Option<BlockInfo> {
        self.block_info
    }
}

#[async_trait]
impl OriginAdvancer for MockChannelReaderProvider {
    async fn advance_origin(&mut self) -> StageResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ChannelReaderProvider for MockChannelReaderProvider {
    async fn next_data(&mut self) -> StageResult<Option<Bytes>> {
        self.data.pop().unwrap_or(Err(StageError::Eof))
    }
}

#[async_trait]
impl ResettableStage for MockChannelReaderProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> StageResult<()> {
        Ok(())
    }
}
