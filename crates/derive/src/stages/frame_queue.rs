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
                    // TODO: what do we do with frame parsing errors?
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::stages::l1_traversal::tests::new_test_traversal;
    use crate::traits::test_utils::TestDAP;
    use crate::DERIVATION_VERSION_0;
    use alloc::vec;
    use alloc::vec::Vec;
    use alloy_primitives::Bytes;

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
        let traversal = new_test_traversal(true, true);
        let results = vec![Ok(Bytes::from(vec![0x00]))];
        let dap = TestDAP { results };
        let retrieval = L1Retrieval::new(traversal, dap);
        let mut frame_queue = FrameQueue::new(retrieval);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, anyhow!("Not enough data").into());
    }

    #[tokio::test]
    async fn test_frame_queue_no_frames_decoded() {
        let traversal = new_test_traversal(true, true);
        let results = vec![Err(StageError::Eof), Ok(Bytes::default())];
        let dap = TestDAP { results };
        let retrieval = L1Retrieval::new(traversal, dap);
        let mut frame_queue = FrameQueue::new(retrieval);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, anyhow!("Not enough data").into());
    }

    #[tokio::test]
    async fn test_frame_queue_wrong_derivation_version() {
        let traversal = new_test_traversal(true, true);
        let results = vec![Ok(Bytes::from(vec![0x01]))];
        let dap = TestDAP { results };
        let retrieval = L1Retrieval::new(traversal, dap);
        let mut frame_queue = FrameQueue::new(retrieval);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, anyhow!("Unsupported derivation version").into());
    }

    #[tokio::test]
    async fn test_frame_queue_frame_too_short() {
        let traversal = new_test_traversal(true, true);
        let results = vec![Ok(Bytes::from(vec![0x00, 0x01]))];
        let dap = TestDAP { results };
        let retrieval = L1Retrieval::new(traversal, dap);
        let mut frame_queue = FrameQueue::new(retrieval);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, anyhow!("Frame too short to decode").into());
    }

    #[tokio::test]
    async fn test_frame_queue_single_frame() {
        let data = new_encoded_test_frames(1);
        let traversal = new_test_traversal(true, true);
        let dap = TestDAP {
            results: vec![Ok(data)],
        };
        let retrieval = L1Retrieval::new(traversal, dap);
        let mut frame_queue = FrameQueue::new(retrieval);
        let frame_decoded = frame_queue.next_frame().await.unwrap();
        let frame = new_test_frames(1);
        assert_eq!(frame[0], frame_decoded);
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, anyhow!("Not enough data").into());
    }

    #[tokio::test]
    async fn test_frame_queue_multiple_frames() {
        let data = new_encoded_test_frames(3);
        let traversal = new_test_traversal(true, true);
        let dap = TestDAP {
            results: vec![Ok(data)],
        };
        let retrieval = L1Retrieval::new(traversal, dap);
        let mut frame_queue = FrameQueue::new(retrieval);
        for i in 0..3 {
            let frame_decoded = frame_queue.next_frame().await.unwrap();
            assert_eq!(frame_decoded.number, i);
        }
        let err = frame_queue.next_frame().await.unwrap_err();
        assert_eq!(err, anyhow!("Not enough data").into());
    }
}
