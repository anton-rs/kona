use crate::{
    errors::PipelineErrorKind,
    metrics::PipelineMetrics,
    pipeline::{Signal, StepResult},
    traits::{
        DerivationPipelineMetrics, FrameQueueMetrics, L1RetrievalMetrics, L1TraversalMetrics,
    },
};
use alloc::sync::Arc;

impl PipelineMetrics {
    /// No-op implementation for `PipelineMetrics`.
    pub fn no_op() -> Self {
        Self {
            derivation_pipeline_metrics: Arc::new(NoopDerivationPipelineMetrics),
            l1_traversal_metrics: Arc::new(NoopL1TraversalMetrics),
            l1_retrieval_metrics: Arc::new(NoopL1RetrievalMetrics),
            frame_queue_metrics: Arc::new(NoopFrameQueueMetrics),
            // todo: add more metrics here for each stage
        }
    }
}

/// No-op implementation of `DerivationPipelineMetrics`.
#[derive(Debug)]
struct NoopDerivationPipelineMetrics;

impl DerivationPipelineMetrics for NoopDerivationPipelineMetrics {
    fn record_step_result(&self, _result: &StepResult) {
        // No-op
    }

    fn record_signal(&self, _signal: &Signal) {
        // No-op
    }
}

/// No-op implementation of `L1TraversalMetrics`.
#[derive(Debug)]
struct NoopL1TraversalMetrics;

impl L1TraversalMetrics for NoopL1TraversalMetrics {
    fn record_block_processed(&self, _block_number: u64) {
        // No-op
    }

    fn record_system_config_update(&self) {
        // No-op
    }

    fn record_reorg_detected(&self) {
        // No-op
    }

    fn record_holocene_activation(&self) {
        // No-op
    }

    fn record_error(&self, _error: &PipelineErrorKind) {
        // No-op
    }
}

/// No-op implementation of `L1RetrievalMetrics`.
#[derive(Debug)]
struct NoopL1RetrievalMetrics;

impl L1RetrievalMetrics for NoopL1RetrievalMetrics {
    fn record_data_fetch_attempt(&self, _block_number: u64) {
        // No-op
    }

    fn record_data_fetch_success(&self, _block_number: u64) {
        // No-op
    }

    fn record_data_fetch_failure(&self, _block_number: u64, _error: &PipelineErrorKind) {
        // No-op
    }

    fn record_block_processed(&self, _block_number: u64) {
        // No-op
    }

    fn record_error(&self, _error: &PipelineErrorKind) {
        // No-op
    }
}

/// No-op implementation of `FrameQueueMetrics`.
#[derive(Debug)]
struct NoopFrameQueueMetrics;

impl FrameQueueMetrics for NoopFrameQueueMetrics {
    fn record_frames_decoded(&self, _count: usize) {
        // No-op
    }

    fn record_frames_dropped(&self, _count: usize) {
        // No-op
    }

    fn record_frames_queued(&self, _count: usize) {
        // No-op
    }

    fn record_load_frames_attempt(&self) {
        // No-op
    }

    fn record_error(&self, _error: &PipelineErrorKind) {
        // No-op
    }
}
