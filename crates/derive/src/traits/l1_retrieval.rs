use crate::errors::{PipelineErrorKind, PipelineResult};
use alloc::boxed::Box;
use alloy_primitives::Address;
use async_trait::async_trait;
use op_alloy_protocol::BlockInfo;

/// Provides L1 blocks for the [L1Retrieval] stage.
/// This is the previous stage in the pipeline.
#[async_trait]
pub trait L1RetrievalProvider {
    /// Returns the next L1 [BlockInfo] in the [L1Traversal] stage, if the stage is not complete.
    /// This function can only be called once while the stage is in progress, and will return
    /// [`None`] on subsequent calls unless the stage is reset or complete. If the stage is
    /// complete and the [BlockInfo] has been consumed, an [PipelineError::Eof] error is returned.
    ///
    /// [L1Traversal]: crate::stages::L1Traversal
    async fn next_l1_block(&mut self) -> PipelineResult<Option<BlockInfo>>;

    /// Returns the batcher [Address] from the [op_alloy_genesis::SystemConfig].
    fn batcher_addr(&self) -> Address;
}

/// Metrics trait for `L1Retrieval`.
pub trait L1RetrievalMetrics: Send + Sync {
    /// Records the number of data fetch attempts.
    fn record_data_fetch_attempt(&self, block_number: u64);
    /// Records successful data fetches.
    fn record_data_fetch_success(&self, block_number: u64);
    /// Records failed data fetches.
    fn record_data_fetch_failure(&self, block_number: u64, error: &PipelineErrorKind);
    /// Records the number of blocks processed.
    fn record_block_processed(&self, block_number: u64);
    /// Records errors.
    fn record_error(&self, error: &PipelineErrorKind);
}
