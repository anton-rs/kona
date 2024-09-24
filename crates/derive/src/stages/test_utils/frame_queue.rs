//! Mock types for the [FrameQueue] stage.

use crate::{
    errors::{PipelineError, PipelineResult},
    stages::FrameQueueProvider,
    traits::{OriginAdvancer, OriginProvider, ResetType, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use op_alloy_protocol::BlockInfo;

/// A mock [FrameQueueProvider] for testing the [FrameQueue] stage.
#[derive(Debug, Default)]
pub struct MockFrameQueueProvider {
    /// The data to return.
    pub data: Vec<PipelineResult<Bytes>>,
}

impl MockFrameQueueProvider {
    /// Creates a new [MockFrameQueueProvider] with the given data.
    pub fn new(data: Vec<PipelineResult<Bytes>>) -> Self {
        Self { data }
    }
}

impl OriginProvider for MockFrameQueueProvider {
    fn origin(&self) -> Option<BlockInfo> {
        None
    }
}

#[async_trait]
impl OriginAdvancer for MockFrameQueueProvider {
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait]
impl FrameQueueProvider for MockFrameQueueProvider {
    type Item = Bytes;

    async fn next_data(&mut self) -> PipelineResult<Self::Item> {
        self.data.pop().unwrap_or(Err(PipelineError::Eof.temp()))
    }
}

#[async_trait]
impl ResettableStage for MockFrameQueueProvider {
    async fn reset(&mut self, _: &ResetType<'_>) -> PipelineResult<()> {
        Ok(())
    }
}
