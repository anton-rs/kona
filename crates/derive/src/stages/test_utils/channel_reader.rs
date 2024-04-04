//! Test [ChannelReader] utilities and mock implementation.

use alloc::vec::Vec;
use alloc::boxed::Box;
use async_trait::async_trait;
use crate::stages::BatchQueueProvider;
use crate::traits::OriginProvider;
use crate::types::{StageResult, BlockInfo, StageError, Batch};

/// A mock implementation of [ChannelReader] for testing purposes.
#[derive(Debug, Default)]
pub struct MockChannelReader {
    /// The list of batches to return.
    pub batches: Vec<StageResult<Batch>>,
}

#[async_trait]
impl BatchQueueProvider for MockChannelReader {
    async fn next_batch(&mut self) -> StageResult<Batch> {
        self.batches.pop().unwrap_or(Err(StageError::NotEnoughData))
    }
}

impl OriginProvider for MockChannelReader {
    fn origin(&self) -> Option<&BlockInfo> {
        None
    }
}
