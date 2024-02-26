//! This module contains the [FrameQueue] stage of the derivation pipeline.

use core::fmt::Debug;

use super::l1_retrieval::L1Retrieval;
use crate::{
    traits::{ChainProvider, DataAvailabilityProvider, ResettableStage},
    types::{BlockInfo, Frame, StageError, StageResult, SystemConfig},
};
use alloc::{boxed::Box, collections::VecDeque};
use anyhow::anyhow;
use async_trait::async_trait;

/// The frame queue stage of the derivation pipeline.
#[derive(Debug)]
pub struct FrameQueue<DAP, CP>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
{
    /// The previous stage in the pipeline.
    pub prev: L1Retrieval<DAP, CP>,
    /// The current frame queue.
    queue: VecDeque<Frame>,
}

impl<DAP, CP> FrameQueue<DAP, CP>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
{
    /// Create a new frame queue stage.
    pub fn new(prev: L1Retrieval<DAP, CP>) -> Self {
        Self {
            prev,
            queue: VecDeque::new(),
        }
    }

    /// Returns the L1 origin [BlockInfo].
    pub fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }

    /// Fetches the next frame from the frame queue.
    pub async fn next_frame(&mut self) -> StageResult<Frame> {
        if self.queue.is_empty() {
            match self.prev.next_data().await {
                Ok(data) => {
                    if let Ok(frames) = Frame::parse_frames(data.as_ref()) {
                        self.queue.extend(frames);
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Error fetching next data: {e}").into());
                }
            }
        }
        // If we did not add more frames but still have more data, retry this function.
        if self.queue.is_empty() {
            return Err(anyhow!("Not enough data").into());
        }

        self.queue
            .pop_front()
            .ok_or_else(|| anyhow!("Frame queue is impossibly empty.").into())
    }
}

#[async_trait]
impl<DAP, CP> ResettableStage for FrameQueue<DAP, CP>
where
    DAP: DataAvailabilityProvider + Send + Debug,
    CP: ChainProvider + Send + Debug,
{
    async fn reset(&mut self, _: BlockInfo, _: SystemConfig) -> StageResult<()> {
        self.queue = VecDeque::default();
        Err(StageError::Eof)
    }
}
