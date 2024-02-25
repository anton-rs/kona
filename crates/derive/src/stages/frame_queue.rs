//! This module contains the [FrameQueue] stage of the derivation pipeline.

use super::l1_retrieval::L1Retrieval;
use crate::{
    traits::{ChainProvider, DataAvailabilityProvider, ResettableStage},
    types::{BlockInfo, Frame, SystemConfig},
};
use alloc::{boxed::Box, collections::VecDeque};
use alloy_primitives::Bytes;
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;

pub struct FrameQueue<DAP, CP>
where
    DAP: DataAvailabilityProvider,
    CP: ChainProvider,
{
    /// The previous stage in the pipeline.
    pub prev: L1Retrieval<DAP, CP>,
    /// The current frame queue.
    queue: VecDeque<Frame>,
}

impl<DAP, CP> FrameQueue<DAP, CP>
where
    DAP: DataAvailabilityProvider,
    CP: ChainProvider,
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
    pub async fn next_frame(&mut self) -> Result<Frame> {
        if self.queue.is_empty() {
            match self.prev.next_data().await {
                Ok(data) => {
                    if let Ok(frames) = Frame::parse_frames(data.as_ref()) {
                        self.queue.extend(frames);
                    }
                }
                Err(e) => {
                    bail!("Error fetching next data: {e}")
                }
            }
        }
        // If we did not add more frames but still have more data, retry this function.
        if self.queue.is_empty() {
            bail!("Not enough data");
        }

        self.queue
            .pop_front()
            .ok_or_else(|| anyhow!("Frame queue is impossibly empty."))
    }
}

#[async_trait]
impl<DAP, CP> ResettableStage for FrameQueue<DAP, CP>
where
    DAP: DataAvailabilityProvider + Send,
    CP: ChainProvider + Send,
{
    async fn reset(&mut self, base: BlockInfo, cfg: SystemConfig) -> Result<()> {
        self.queue = VecDeque::default();
        Ok(())
    }
}
