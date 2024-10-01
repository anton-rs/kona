//! This module contains the [FrameQueue] stage of the derivation pipeline.

use crate::{
    errors::{PipelineError, PipelineResult},
    stages::ChannelBankProvider,
    traits::{OriginAdvancer, OriginProvider, PreviousStage, ResettableStage},
};
use alloc::{boxed::Box, collections::VecDeque, sync::Arc};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use core::fmt::Debug;
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

    /// Returns if holocene is active.
    pub fn is_holocene_active(&self, origin: BlockInfo) -> bool {
        self.rollup_config.is_holocene_active(origin.timestamp)
    }

    /// Prunes frames if Holocene is active.
    pub fn prune(&mut self, origin: BlockInfo) {
        if !self.is_holocene_active(origin) {
            return;
        }

        let mut i = 0;
        while i < self.queue.len() - 1 {
            let prev_frame = &self.queue[i];
            let next_frame = &self.queue[i + 1];
            let extends_channel = prev_frame.id == next_frame.id;

            // If the frames are in the same channel, and the frame numbers are not sequential,
            // drop the next frame.
            if extends_channel && prev_frame.number + 1 != next_frame.number {
                self.queue.remove(i + 1);
                continue;
            }

            // If the frames are in the same channel, and the previous is last, drop the next frame.
            if extends_channel && prev_frame.is_last {
                self.queue.remove(i + 1);
                continue;
            }

            // If the frames are in different channels, the next frame must be first.
            if !extends_channel && next_frame.number != 0 {
                self.queue.remove(i + 1);
                continue;
            }

            // If the frames are in different channels, and the current channel is not last, walk
            // back the channel and drop all prev frames.
            if !extends_channel && !prev_frame.is_last && next_frame.number == 0 {
                // Find the index of the first frame in the queue with the same channel ID
                // as the previous frame.
                let first_frame =
                    self.queue.iter().position(|f| f.id == prev_frame.id).expect("infallible");

                // Drain all frames from the previous channel.
                let drained = self.queue.drain(first_frame..=i);
                i = i.saturating_sub(drained.len());
                continue;
            }

            i += 1;
        }
    }

    /// Loads more frames into the [FrameQueue].
    pub async fn load_frames(&mut self) -> PipelineResult<()> {
        // Skip loading frames if the queue is not empty.
        if !self.queue.is_empty() {
            return Ok(());
        }

        let data = match self.prev.next_data().await {
            Ok(data) => data,
            Err(e) => {
                debug!(target: "frame-queue", "Failed to retrieve data: {:?}", e);
                // SAFETY: Bubble up potential EOF error without wrapping.
                return Err(e);
            }
        };

        let Ok(frames) = Frame::parse_frames(&data.into()) else {
            crate::inc!(DERIVED_FRAMES_COUNT, &["failed"]);
            // There may be more frames in the queue for the
            // pipeline to advance, so don't return an error here.
            error!(target: "frame-queue", "Failed to parse frames from data.");
            return Ok(());
        };

        // Optimistically extend the queue with the new frames.
        self.queue.extend(frames);

        // Prune frames if Holocene is active.
        let origin = self.origin().ok_or(PipelineError::MissingOrigin.crit())?;
        self.prune(origin);

        crate::inc!(DERIVED_FRAMES_COUNT, self.queue.len() as f64, &["success"]);

        Ok(())
    }
}

impl<P> PreviousStage for FrameQueue<P>
where
    P: FrameQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    type Previous = P;

    fn prev(&self) -> Option<&Self::Previous> {
        Some(&self.prev)
    }

    fn prev_mut(&mut self) -> Option<&mut Self::Previous> {
        Some(&mut self.prev)
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

    pub(crate) fn encode_frames(frames: Vec<Frame>) -> Bytes {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[DERIVATION_VERSION_0]);
        for frame in frames.iter() {
            bytes.extend_from_slice(&frame.encode());
        }
        Bytes::from(bytes)
    }

    pub(crate) fn new_encoded_test_frames(count: usize) -> Bytes {
        let frames = new_test_frames(count);
        encode_frames(frames)
    }

    #[tokio::test]
    async fn test_frame_queue_empty_bytes() {
        let data = vec![Ok(Bytes::from(vec![0x00]))];
        let mut mock = MockFrameQueueProvider::new(data);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Default::default());
        assert!(!frame_queue.is_holocene_active(BlockInfo::default()));
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::NotEnoughData.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_no_frames_decoded() {
        let data = vec![Err(PipelineError::Eof.temp()), Ok(Bytes::default())];
        let mut mock = MockFrameQueueProvider::new(data);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Default::default());
        assert!(!frame_queue.is_holocene_active(BlockInfo::default()));
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::NotEnoughData.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_wrong_derivation_version() {
        let data = vec![Ok(Bytes::from(vec![0x01]))];
        let mut mock = MockFrameQueueProvider::new(data);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Default::default());
        assert!(!frame_queue.is_holocene_active(BlockInfo::default()));
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::NotEnoughData.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_frame_too_short() {
        let data = vec![Ok(Bytes::from(vec![0x00, 0x01]))];
        let mut mock = MockFrameQueueProvider::new(data);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Default::default());
        assert!(!frame_queue.is_holocene_active(BlockInfo::default()));
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::NotEnoughData.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_single_frame() {
        let data = new_encoded_test_frames(1);
        let mut mock = MockFrameQueueProvider::new(vec![Ok(data)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Default::default());
        assert!(!frame_queue.is_holocene_active(BlockInfo::default()));
        let frame_decoded = frame_queue.next_frame().await.unwrap();
        let frame = new_test_frames(1);
        assert_eq!(frame[0], frame_decoded);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_multiple_frames() {
        let data = new_encoded_test_frames(3);
        let mut mock = MockFrameQueueProvider::new(vec![Ok(data)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Default::default());
        assert!(!frame_queue.is_holocene_active(BlockInfo::default()));
        for i in 0..3 {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded.number, i);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_frame_queue_missing_origin() {
        let data = new_encoded_test_frames(1);
        let mock = MockFrameQueueProvider::new(vec![Ok(data)]);
        let mut frame_queue = FrameQueue::new(mock, Default::default());
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::MissingOrigin.crit());
    }

    #[tokio::test]
    async fn test_holocene_valid_frames() {
        let channel = new_encoded_test_frames(3);
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(channel)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for i in 0..3 {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded.number, i);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_holocene_unordered_frames() {
        let frames = vec![
            // -- First Channel --
            Frame { id: [0xEE; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 2, data: vec![0xDD; 50], is_last: true },
            // Frame with the same channel id, but after is_last should be dropped.
            Frame { id: [0xEE; 16], number: 3, data: vec![0xDD; 50], is_last: false },
            // -- Next Channel --
            Frame { id: [0xFF; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xFF; 16], number: 1, data: vec![0xDD; 50], is_last: true },
        ];
        let encoded = encode_frames(frames.clone());
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(encoded)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for frame in frames.iter().take(3) {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, *frame);
        }
        for i in 0..2 {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, frames[i + 4]);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_holocene_non_sequential_frames() {
        let frames = vec![
            // -- First Channel --
            Frame { id: [0xEE; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            // Both this and the next frames should be dropped since neither will be
            // interpreted as having the next sequential frame number after 1.
            Frame { id: [0xEE; 16], number: 3, data: vec![0xDD; 50], is_last: true },
            Frame { id: [0xEE; 16], number: 4, data: vec![0xDD; 50], is_last: false },
        ];
        let encoded = encode_frames(frames.clone());
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(encoded)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for frame in frames.iter().take(2) {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, *frame);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_holocene_unclosed_channel() {
        let frames = vec![
            // -- First Channel --
            // Since this channel isn't closed by a last frame it is entirely dropped
            Frame { id: [0xEE; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 2, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 3, data: vec![0xDD; 50], is_last: false },
            // -- Next Channel --
            Frame { id: [0xFF; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xFF; 16], number: 1, data: vec![0xDD; 50], is_last: true },
        ];
        let encoded = encode_frames(frames.clone());
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(encoded)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for i in 0..2 {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, frames[i + 4]);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_holocene_unstarted_channel() {
        let frames = vec![
            // -- First Channel --
            Frame { id: [0xDD; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xDD; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xDD; 16], number: 2, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xDD; 16], number: 3, data: vec![0xDD; 50], is_last: true },
            // -- Second Channel --
            // Since this channel doesn't have a starting frame where number == 0,
            // it is entirely dropped.
            Frame { id: [0xEE; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 2, data: vec![0xDD; 50], is_last: true },
            // -- Third Channel --
            Frame { id: [0xFF; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xFF; 16], number: 1, data: vec![0xDD; 50], is_last: true },
        ];
        let encoded = encode_frames(frames.clone());
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(encoded)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for frame in frames.iter().take(4) {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, *frame);
        }
        for i in 0..2 {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, frames[i + 6]);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    // Notice: The first channel is **not** dropped here because there can still be
    // frames that come in to successfully close the channel.
    #[tokio::test]
    async fn test_holocene_unclosed_channel_with_invalid_start() {
        let frames = vec![
            // -- First Channel --
            Frame { id: [0xEE; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 2, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 3, data: vec![0xDD; 50], is_last: false },
            // -- Next Channel --
            // This is also an invalid channel because it is never started
            // since there isn't a first frame with number == 0
            Frame { id: [0xFF; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xFF; 16], number: 2, data: vec![0xDD; 50], is_last: true },
        ];
        let encoded = encode_frames(frames.clone());
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(encoded)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for frame in frames.iter().take(4) {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, *frame);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_holocene_replace_channel() {
        let frames = vec![
            // -- First Channel - VALID & CLOSED --
            Frame { id: [0xDD; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xDD; 16], number: 1, data: vec![0xDD; 50], is_last: true },
            // -- Second Channel - VALID & NOT CLOSED / DROPPED --
            Frame { id: [0xEE; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xEE; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            // -- Third Channel - VALID & CLOSED / REPLACES CHANNEL #2 --
            Frame { id: [0xFF; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xFF; 16], number: 1, data: vec![0xDD; 50], is_last: true },
        ];
        let encoded = encode_frames(frames.clone());
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(encoded)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for frame in frames.iter().filter(|f| f.id != [0xEE; 16]) {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, *frame);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_holocene_interleaved_invalid_channel() {
        let frames = vec![
            // -- First channel is dropped since it is replaced by the second channel --
            // -- Second channel is dropped since it isn't closed --
            Frame { id: [0x01; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0x02; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0x01; 16], number: 1, data: vec![0xDD; 50], is_last: true },
            Frame { id: [0x02; 16], number: 1, data: vec![0xDD; 50], is_last: false },
            // -- Third Channel - VALID & CLOSED --
            Frame { id: [0xFF; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xFF; 16], number: 1, data: vec![0xDD; 50], is_last: true },
        ];
        let encoded = encode_frames(frames.clone());
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(encoded)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for frame in frames[4..].iter() {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, *frame);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_holocene_interleaved_valid_channel() {
        let frames = vec![
            // -- First channel is dropped since it is replaced by the second channel --
            // -- Second channel is successfully closed so it's valid --
            Frame { id: [0x01; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0x02; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0x01; 16], number: 1, data: vec![0xDD; 50], is_last: true },
            Frame { id: [0x02; 16], number: 1, data: vec![0xDD; 50], is_last: true },
            // -- Third Channel - VALID & CLOSED --
            Frame { id: [0xFF; 16], number: 0, data: vec![0xDD; 50], is_last: false },
            Frame { id: [0xFF; 16], number: 1, data: vec![0xDD; 50], is_last: true },
        ];
        let encoded = encode_frames(frames.clone());
        let config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let mut mock = MockFrameQueueProvider::new(vec![Ok(encoded)]);
        mock.set_origin(BlockInfo::default());
        let mut frame_queue = FrameQueue::new(mock, Arc::new(config));
        assert!(frame_queue.is_holocene_active(BlockInfo::default()));
        for frame in [&frames[1], &frames[3], &frames[4], &frames[5]].iter() {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded, **frame);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
    }
}
