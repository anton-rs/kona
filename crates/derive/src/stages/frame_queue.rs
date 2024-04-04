//! This module contains the [FrameQueue] stage of the derivation pipeline.

use core::fmt::Debug;

use crate::{
    stages::ChannelBankProvider,
    traits::{LogLevel, OriginProvider, ResettableStage, TelemetryProvider},
    types::{into_frames, BlockInfo, Frame, StageError, StageResult, SystemConfig},
};
use alloc::{boxed::Box, collections::VecDeque};
use alloy_primitives::Bytes;
use anyhow::anyhow;
use async_trait::async_trait;

/// Provides data frames for the [FrameQueue] stage.
#[async_trait]
pub trait FrameQueueProvider {
    /// An item that can be converted into a byte array.
    type Item: Into<Bytes>;

    /// Retrieves the next data item from the L1 retrieval stage.
    /// If there is data, it pushes it into the next stage.
    /// If there is no data, it returns an error.
    async fn next_data(&mut self) -> StageResult<Self::Item>;
}

/// The [FrameQueue] stage of the derivation pipeline.
/// This stage takes the output of the [L1Retrieval] stage and parses it into frames.
#[derive(Debug)]
pub struct FrameQueue<P, T>
where
    P: FrameQueueProvider + OriginProvider + Debug,
    T: TelemetryProvider + Debug,
{
    /// The previous stage in the pipeline.
    pub prev: P,
    /// Telemetry
    pub telemetry: T,
    /// The current frame queue.
    queue: VecDeque<Frame>,
}

impl<P, T> FrameQueue<P, T>
where
    P: FrameQueueProvider + OriginProvider + Debug,
    T: TelemetryProvider + Debug,
{
    /// Create a new [FrameQueue] stage with the given previous [L1Retrieval] stage.
    pub fn new(prev: P, telemetry: T) -> Self {
        Self { prev, telemetry, queue: VecDeque::new() }
    }
}

#[async_trait]
impl<P, T> ChannelBankProvider for FrameQueue<P, T>
where
    P: FrameQueueProvider + OriginProvider + Send + Debug,
    T: TelemetryProvider + Send + Debug,
{
    async fn next_frame(&mut self) -> StageResult<Frame> {
        if self.queue.is_empty() {
            match self.prev.next_data().await {
                Ok(data) => {
                    if let Ok(frames) = into_frames(Ok(data)) {
                        self.queue.extend(frames);
                    } else {
                        // TODO: log parsing frame error
                        // Failed to parse frames, but there may be more frames in the queue for
                        //
                        // the pipeline to advance, so don't return an error here.
                    }
                }
                Err(e) => {
                    // TODO: log retrieval error
                    // The error must be bubbled up without a wrapper in case it's an EOF error.
                    return Err(e);
                }
            }
        }

        // If we did not add more frames but still have more data, retry this function.
        if self.queue.is_empty() {
            self.telemetry.write(
                alloy_primitives::Bytes::from(
                    "Queue is empty after fetching data. Retrying next_frame.",
                ),
                LogLevel::Debug,
            );
            return Err(StageError::NotEnoughData);
        }

        self.queue.pop_front().ok_or_else(|| anyhow!("Frame queue is impossibly empty.").into())
    }
}

impl<P, T> OriginProvider for FrameQueue<P, T>
where
    P: FrameQueueProvider + OriginProvider + Debug,
    T: TelemetryProvider + Debug,
{
    fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P, T> ResettableStage for FrameQueue<P, T>
where
    P: FrameQueueProvider + OriginProvider + Send + Debug,
    T: TelemetryProvider + Send + Debug,
{
    async fn reset(&mut self, _: BlockInfo, _: SystemConfig) -> StageResult<()> {
        self.queue = VecDeque::default();
        Err(StageError::Eof)
    }
}

// #[cfg(test)]
// pub(crate) mod tests {
//     use super::*;
//     use crate::{
//         // stages::l1_traversal::tests::new_populated_test_traversal,
//         // traits::test_utils::{TestDAP, TestTelemetry},
//         DERIVATION_VERSION_0,
//     };
//     use alloc::{vec, vec::Vec};
//     use alloy_primitives::Bytes;
//
//     pub(crate) fn new_test_frames(count: usize) -> Vec<Frame> {
//         (0..count)
//             .map(|i| Frame {
//                 id: [0xFF; 16],
//                 number: i as u16,
//                 data: vec![0xDD; 50],
//                 is_last: i == count - 1,
//             })
//             .collect()
//     }
//
//     pub(crate) fn new_encoded_test_frames(count: usize) -> Bytes {
//         let frames = new_test_frames(count);
//         let mut bytes = Vec::new();
//         bytes.extend_from_slice(&[DERIVATION_VERSION_0]);
//         for frame in frames.iter() {
//             bytes.extend_from_slice(&frame.encode());
//         }
//         Bytes::from(bytes)
//     }
//
//     #[tokio::test]
//     async fn test_frame_queue_empty_bytes() {
//         let telemetry = TestTelemetry::new();
//         let traversal = new_populated_test_traversal();
//         let results = vec![Ok(Bytes::from(vec![0x00]))];
//         let dap = TestDAP { results };
//         let retrieval = L1Retrieval::new(traversal, dap, TestTelemetry::new());
//         let mut frame_queue = FrameQueue::new(retrieval, telemetry);
//         let err = frame_queue.next_frame().await.unwrap_err();
//         assert_eq!(err, StageError::NotEnoughData);
//     }
//
//     #[tokio::test]
//     async fn test_frame_queue_no_frames_decoded() {
//         let telemetry = TestTelemetry::new();
//         let traversal = new_populated_test_traversal();
//         let results = vec![Err(StageError::Eof), Ok(Bytes::default())];
//         let dap = TestDAP { results };
//         let retrieval = L1Retrieval::new(traversal, dap, TestTelemetry::new());
//         let mut frame_queue = FrameQueue::new(retrieval, telemetry);
//         let err = frame_queue.next_frame().await.unwrap_err();
//         assert_eq!(err, StageError::NotEnoughData);
//     }
//
//     #[tokio::test]
//     async fn test_frame_queue_wrong_derivation_version() {
//         let telemetry = TestTelemetry::new();
//         let traversal = new_populated_test_traversal();
//         let results = vec![Ok(Bytes::from(vec![0x01]))];
//         let dap = TestDAP { results };
//         let retrieval = L1Retrieval::new(traversal, dap, TestTelemetry::new());
//         let mut frame_queue = FrameQueue::new(retrieval, telemetry);
//         let err = frame_queue.next_frame().await.unwrap_err();
//         assert_eq!(err, StageError::NotEnoughData);
//     }
//
//     #[tokio::test]
//     async fn test_frame_queue_frame_too_short() {
//         let telemetry = TestTelemetry::new();
//         let traversal = new_populated_test_traversal();
//         let results = vec![Ok(Bytes::from(vec![0x00, 0x01]))];
//         let dap = TestDAP { results };
//         let retrieval = L1Retrieval::new(traversal, dap, TestTelemetry::new());
//         let mut frame_queue = FrameQueue::new(retrieval, telemetry);
//         let err = frame_queue.next_frame().await.unwrap_err();
//         assert_eq!(err, StageError::NotEnoughData);
//     }
//
//     #[tokio::test]
//     async fn test_frame_queue_single_frame() {
//         let data = new_encoded_test_frames(1);
//         let telemetry = TestTelemetry::new();
//         let traversal = new_populated_test_traversal();
//         let dap = TestDAP { results: vec![Ok(data)] };
//         let retrieval = L1Retrieval::new(traversal, dap, TestTelemetry::new());
//         let mut frame_queue = FrameQueue::new(retrieval, telemetry);
//         let frame_decoded = frame_queue.next_frame().await.unwrap();
//         let frame = new_test_frames(1);
//         assert_eq!(frame[0], frame_decoded);
//         let err = frame_queue.next_frame().await.unwrap_err();
//         assert_eq!(err, StageError::Eof);
//     }
//
//     #[tokio::test]
//     async fn test_frame_queue_multiple_frames() {
//         let telemetry = TestTelemetry::new();
//         let data = new_encoded_test_frames(3);
//         let traversal = new_populated_test_traversal();
//         let dap = TestDAP { results: vec![Ok(data)] };
//         let retrieval = L1Retrieval::new(traversal, dap, TestTelemetry::new());
//         let mut frame_queue = FrameQueue::new(retrieval, telemetry);
//         for i in 0..3 {
//             let frame_decoded = frame_queue.next_frame().await.unwrap();
//             assert_eq!(frame_decoded.number, i);
//         }
//         let err = frame_queue.next_frame().await.unwrap_err();
//         assert_eq!(err, StageError::Eof);
//     }
// }
//
