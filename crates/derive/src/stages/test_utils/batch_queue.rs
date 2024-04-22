//! A mock implementation of the [`BatchQueue`] stage for testing.

use crate::{
    stages::batch_queue::BatchQueueProvider,
    traits::{OriginProvider, PreviousStage, ResettableStage},
    types::{Batch, BlockInfo, StageError, StageResult, SystemConfig},
};
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;

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
    fn origin(&self) -> Option<&BlockInfo> {
        self.origin.as_ref()
    }
}

#[async_trait]
impl BatchQueueProvider for MockBatchQueueProvider {
    async fn next_batch(&mut self) -> StageResult<Batch> {
        self.batches.pop().ok_or(StageError::Eof)?
    }
}

#[async_trait]
impl ResettableStage for MockBatchQueueProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> StageResult<()> {
        Ok(())
    }
}

impl PreviousStage for MockBatchQueueProvider {
    type Previous = MockBatchQueueProvider;

    fn previous(&self) -> Option<&Self::Previous> {
        Some(self)
    }
}
