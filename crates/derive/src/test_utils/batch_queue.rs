//! A mock implementation of the [`BatchQueue`] stage for testing.

use crate::{
    batch::Batch,
    errors::{PipelineError, PipelineResult},
    stages::BatchQueueProvider,
    traits::{FlushableStage, OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};

/// A mock provider for the [BatchQueue] stage.
#[derive(Debug, Default)]
pub struct TestBatchQueueProvider {
    /// The origin of the L1 block.
    pub origin: Option<BlockInfo>,
    /// A list of batches to return.
    pub batches: Vec<PipelineResult<Batch>>,
    /// Tracks if the provider has been flushed.
    pub flushed: bool,
    /// Tracks if the reset method was called.
    pub reset: bool,
}

impl TestBatchQueueProvider {
    /// Creates a new [MockBatchQueueProvider] with the given origin and batches.
    pub fn new(batches: Vec<PipelineResult<Batch>>) -> Self {
        Self { origin: Some(BlockInfo::default()), batches, flushed: false, reset: false }
    }
}

impl OriginProvider for TestBatchQueueProvider {
    fn origin(&self) -> Option<BlockInfo> {
        self.origin
    }
}

#[async_trait]
impl BatchQueueProvider for TestBatchQueueProvider {
    fn flush(&mut self) {
        self.flushed = true;
    }

    async fn next_batch(&mut self, _: L2BlockInfo, _: &[BlockInfo]) -> PipelineResult<Batch> {
        self.batches.pop().ok_or(PipelineError::Eof.temp())?
    }
}

#[async_trait]
impl OriginAdvancer for TestBatchQueueProvider {
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait]
impl FlushableStage for TestBatchQueueProvider {
    async fn flush_channel(&mut self) -> PipelineResult<()> {
        self.flushed = true;
        Ok(())
    }
}

#[async_trait]
impl ResettableStage for TestBatchQueueProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> PipelineResult<()> {
        self.reset = true;
        Ok(())
    }
}
