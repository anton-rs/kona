//! A mock implementation of the [`BatchQueue`] stage for testing.

use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;

use crate::{
    stages::attributes_queue::AttributesProvider,
    traits::OriginProvider,
    types::{BlockInfo, L2BlockInfo, SingleBatch, StageError, StageResult},
};

/// A mock implementation of the [`BatchQueue`] stage for testing.
#[derive(Debug, Default)]
pub struct MockBatchQueue {
    /// The origin of the L1 block.
    origin: Option<BlockInfo>,
    /// A list of batches to return.
    batches: Vec<StageResult<SingleBatch>>,
}

impl OriginProvider for MockBatchQueue {
    fn origin(&self) -> Option<&BlockInfo> {
        self.origin.as_ref()
    }
}

#[async_trait]
impl AttributesProvider for MockBatchQueue {
    async fn next_batch(&mut self, _parent: L2BlockInfo) -> StageResult<SingleBatch> {
        self.batches.pop().ok_or(StageError::Eof)?
    }

    fn is_last_in_span(&self) -> bool {
        self.batches.is_empty()
    }
}

/// Creates a new [`MockBatchQueue`] with the given origin and batches.
pub fn new_mock_batch_queue(
    origin: Option<BlockInfo>,
    batches: Vec<StageResult<SingleBatch>>,
) -> MockBatchQueue {
    MockBatchQueue { origin, batches }
}
