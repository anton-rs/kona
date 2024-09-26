//! A mock implementation of the [`BatchQueue`] stage for testing.

use crate::{
    batch::Batch,
    errors::{PipelineError, PipelineResult},
    stages::batch_queue::BatchQueueProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};

/// A mock provider for the [BatchQueue] stage.
#[derive(Debug, Default)]
pub struct MockBatchQueueProvider {
    /// The origin of the L1 block.
    pub origin: Option<BlockInfo>,
    /// A list of batches to return.
    pub batches: Vec<PipelineResult<Batch>>,
}

impl MockBatchQueueProvider {
    /// Creates a new [MockBatchQueueProvider] with the given origin and batches.
    pub fn new(batches: Vec<PipelineResult<Batch>>) -> Self {
        Self { origin: Some(BlockInfo::default()), batches }
    }
}

impl OriginProvider for MockBatchQueueProvider {
    fn origin(&self) -> Option<BlockInfo> {
        self.origin
    }
}

#[async_trait]
impl BatchQueueProvider for MockBatchQueueProvider {
    fn flush(&mut self) { /* noop */
    }

    async fn next_batch(&mut self, _: L2BlockInfo) -> PipelineResult<Batch> {
        self.batches.pop().ok_or(PipelineError::Eof.temp())?
    }
}

#[async_trait]
impl OriginAdvancer for MockBatchQueueProvider {
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ResettableStage for MockBatchQueueProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> PipelineResult<()> {
        Ok(())
    }
}
