use crate::errors::{PipelineErrorKind, PipelineResult};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;

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

/// Metrics trait for `FrameQueue`.
pub trait FrameQueueMetrics {
    /// Records the number of frames decoded.
    fn record_frames_decoded(&self, count: usize);
    /// Records the number of frames dropped.
    fn record_frames_dropped(&self, count: usize);
    /// Records the number of frames queued.
    fn record_frames_queued(&self, count: usize);
    /// Records the number of frames loaded.
    fn record_load_frames_attempt(&self);
    /// Records error loading frames.
    fn record_error(&self, error: &PipelineErrorKind);
}
