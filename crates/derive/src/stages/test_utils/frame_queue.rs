//! Mock types for the [FrameQueue] stage.

use crate::{
    stages::FrameQueueProvider,
    traits::OriginProvider,
    types::{BlockInfo, StageError, StageResult},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::Bytes;
use async_trait::async_trait;

/// A mock [FrameQueueProvider] for testing the [FrameQueue] stage.
#[derive(Debug)]
pub struct MockFrameQueueProvider {
    /// The data to return.
    pub data: Vec<StageResult<Bytes>>,
}

impl MockFrameQueueProvider {
    /// Creates a new [MockFrameQueueProvider] with the given data.
    pub fn new(data: Vec<StageResult<Bytes>>) -> Self {
        Self { data }
    }
}

impl OriginProvider for MockFrameQueueProvider {
    fn origin(&self) -> Option<&BlockInfo> {
        None
    }
}

#[async_trait]
impl FrameQueueProvider for MockFrameQueueProvider {
    type Item = Bytes;

    async fn next_data(&mut self) -> StageResult<Self::Item> {
        self.data.pop().unwrap_or(Err(StageError::Eof))
    }
}
