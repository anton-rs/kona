//! Test [ChannelReader] utilities and mock implementation.

use crate::{
    stages::BatchQueueProvider,
    traits::OriginProvider,
    types::{Batch, BlockInfo, StageError, StageResult},
};
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;

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
