//! This module contains the [FrameQueue] stage of the derivation pipeline.

use crate::{
    errors::{PipelineError, PipelineResult},
    stages::ChannelBankProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, collections::VecDeque};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use core::fmt::Debug;
use alloc::sync::Arc;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BlockInfo, Frame};
use tracing::{debug, error, trace};

/// Provides data frames for the [FrameQueue] stage.
#[async_trait]
pub trait FrameQueueProvider {
    /// An item that can be converted into a byte array.
    type Item: Into<Bytes>;

    /// Retrieves the next data item from the L1 retrieval stage.
    /// If there is data, it pushes it into the next stage.
    /// If there is no data, it returns an error.
    async fn next_data(&mut self) -> PipelineResult<Self::Item>;
}

/// The [FrameQueue] stage of the derivation pipeline.
/// This stage takes the output of the [L1Retrieval] stage and parses it into frames.
///
/// [L1Retrieval]: crate::stages::L1Retrieval
#[derive(Debug)]
pub struct FrameQueue<P>
where
    P: FrameQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// The previous stage in the pipeline.
    pub prev: P,
    /// The current frame queue.
    queue: VecDeque<Frame>,
    /// The rollup config.
    rollup_config: Arc<RollupConfig>,
}

impl<P> FrameQueue<P>
where
    P: FrameQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// Create a new [FrameQueue] stage with the given previous [L1Retrieval] stage.
    ///
    /// [L1Retrieval]: crate::stages::L1Retrieval
    pub fn new(prev: P, cfg: Arc<RollupConfig>) -> Self {
        crate::set!(STAGE_RESETS, 0, &["frame-queue"]);
        Self { prev, queue: VecDeque::new(), rollup_config: cfg }
    }

    /// Prunes frames if Holocene is active.
    pub fn prune(&mut self) {
        let Some(origin) = self.prev.origin() else {
            warn!(target: "frame-queue", "Failed to get origin");
            return;
        }
        if !self.rollup_config.is_holocene_active(origin.timestamp) {
            return;
        }

        for frame in self.queue {

        }
    }

    /// Loads more frames into the [FrameQueue].
    pub async fn load_frames(&mut self) -> PipelineResult<()> {
        // Skip loading frames if the queue is not empty.
        if !self.queue.is_empty() {
            return Ok(());
        }

        let data = match self.prev.next_data().await else {
            Ok(data) => data,
            Err(e) => {
                debug!(target: "frame-queue", "Failed to retrieve data: {:?}", e);
                // SAFETY: Bubble up potential EOF error without wrapping.
                return Err(e);
            }
        };

        // 
        if let Ok(frames) = Frame::parse_frames(&data.into()) {
            crate::inc!(DERIVED_FRAMES_COUNT, frames.len() as f64, &["success"]);
            self.queue.extend(frames);
        } else {
            crate::inc!(DERIVED_FRAMES_COUNT, &["failed"]);
            // There may be more frames in the queue for the
            // pipeline to advance, so don't return an error here.
            error!(target: "frame-queue", "Failed to parse frames from data.");
            return Ok(());
        }

        // Prune frames if Holocene is active.
        self.prune();

        Ok(())
    }
}

#[async_trait]
impl<P> OriginAdvancer for FrameQueue<P>
where
    P: FrameQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

#[async_trait]
impl<P> ChannelBankProvider for FrameQueue<P>
where
    P: FrameQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn next_frame(&mut self) -> PipelineResult<Frame> {
        self.load_frames().await?;

        // If we did not add more frames but still have more data, retry this function.
        if self.queue.is_empty() {
            trace!(target: "frame-queue", "Queue is empty after fetching data. Retrying next_frame.");
            return Err(PipelineError::NotEnoughData.temp());
        }

        Ok(self.queue.pop_front().expect("Frame queue impossibly empty"))
    }
}

impl<P> OriginProvider for FrameQueue<P>
where
    P: FrameQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> ResettableStage for FrameQueue<P>
where
    P: FrameQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn reset(
        &mut self,
        block_info: BlockInfo,
        system_config: &SystemConfig,
    ) -> PipelineResult<()> {
        self.prev.reset(block_info, system_config).await?;
        self.queue = VecDeque::default();
        crate::inc!(STAGE_RESETS, &["frame-queue"]);
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::stages::test_utils::MockFrameQueueProvider;
    use alloc::{vec, vec::Vec};
    use op_alloy_protocol::DERIVATION_VERSION_0;

    pub(crate) fn new_test_frames(count: usize) -> Vec<Frame> {
        (0..count)
            .map(|i| Frame {
                id: [0xFF; 16],
                number: i as u16,
                data: vec![0xDD; 50],
                is_last: i == count - 1,
            })
            .collect()
    }

    pub(crate) fn new_encoded_test_frames(count: usize) -> Bytes {
        let frames = new_test_frames(count);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[DERIVATION_VERSION_0]);
        for frame in frames.iter() {
            bytes.extend_from_slice(&frame.encode());
        }
        Bytes::from(bytes)
    }

    #[tokio::test]
    async fn test_frame_queue_empty_bytes() {
        let data = vec![Ok(Bytes::from(vec![0x00]))];
        let mock = MockFrameQueueProvider { data };
        let mut frame_queue = FrameQueue::new(mock);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::NotEnoughData.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_no_frames_decoded() {
        let data = vec![Err(PipelineError::Eof.temp()), Ok(Bytes::default())];
        let mock = MockFrameQueueProvider { data };
        let mut frame_queue = FrameQueue::new(mock);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::NotEnoughData.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_wrong_derivation_version() {
        let data = vec![Ok(Bytes::from(vec![0x01]))];
        let mock = MockFrameQueueProvider { data };
        let mut frame_queue = FrameQueue::new(mock);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::NotEnoughData.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_frame_too_short() {
        let data = vec![Ok(Bytes::from(vec![0x00, 0x01]))];
        let mock = MockFrameQueueProvider { data };
        let mut frame_queue = FrameQueue::new(mock);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::NotEnoughData.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_single_frame() {
        let data = new_encoded_test_frames(1);
        let mock = MockFrameQueueProvider { data: vec![Ok(data)] };
        let mut frame_queue = FrameQueue::new(mock);
        let frame_decoded = frame_queue.next_frame().await.unwrap();
        let frame = new_test_frames(1);
        assert_eq!(frame[0], frame_decoded);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_multiple_frames() {
        let data = new_encoded_test_frames(3);
        let mock = MockFrameQueueProvider { data: vec![Ok(data)] };
        let mut frame_queue = FrameQueue::new(mock);
        for i in 0..3 {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded.number, i);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }
}
