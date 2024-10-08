//! Test utilities for the [ChannelReader] stage.

use crate::{
    errors::{PipelineError, PipelineResult},
    stages::ChannelReaderProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::BlockInfo;

/// A mock [ChannelReaderProvider] for testing the [ChannelReader] stage.
#[derive(Debug, Default)]
pub struct MockChannelReaderProvider {
    /// The data to return.
    pub data: Vec<PipelineResult<Option<Bytes>>>,
    /// The origin block info
    pub block_info: Option<BlockInfo>,
    /// Tracks if the channel reader provider has been reset.
    pub reset: bool,
}

impl MockChannelReaderProvider {
    /// Creates a new [MockChannelReaderProvider] with the given data.
    pub fn new(data: Vec<PipelineResult<Option<Bytes>>>) -> Self {
        Self { data, block_info: Some(BlockInfo::default()), reset: false }
    }
}

impl OriginProvider for MockChannelReaderProvider {
    fn origin(&self) -> Option<BlockInfo> {
        self.block_info
    }
}

#[async_trait]
impl OriginAdvancer for MockChannelReaderProvider {
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ChannelReaderProvider for MockChannelReaderProvider {
    async fn next_data(&mut self) -> PipelineResult<Option<Bytes>> {
        self.data.pop().unwrap_or(Err(PipelineError::Eof.temp()))
    }
}

#[async_trait]
impl ResettableStage for MockChannelReaderProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> PipelineResult<()> {
        self.reset = true;
        Ok(())
    }
}
