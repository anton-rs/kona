//! A mock implementation of the [`BatchQueue`] stage for testing.

use crate::{
    batch::Batch,
    errors::{StageError, StageResult},
    stages::batch_queue::BatchQueueProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use kona_primitives::{BlockInfo, SystemConfig};

/// A mock provider for the [BatchQueue] stage.
#[derive(Debug, Default)]
pub struct MockBatchQueueProvider {
    /// The origin of the L1 block.
    pub origin: Option<BlockInfo>,
    /// A list of batches to return.
    pub batches: Vec<StageResult<Batch>>,
}

impl MockBatchQueueProvider {
    /// Creates a new [MockBatchQueueProvider] with the given origin and batches.
    pub fn new(batches: Vec<StageResult<Batch>>) -> Self {
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
    async fn next_batch(&mut self) -> StageResult<Batch> {
        self.batches.pop().ok_or(StageError::Eof)?
    }
}

#[async_trait]
impl OriginAdvancer for MockBatchQueueProvider {
    async fn advance_origin(&mut self) -> StageResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ResettableStage for MockBatchQueueProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> StageResult<()> {
        Ok(())
    }
}
