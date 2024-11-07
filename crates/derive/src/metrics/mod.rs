//! Metrics for the derivation pipeline.

mod noop;

use crate::{
    errors::PipelineErrorKind,
    pipeline::Signal,
    traits::{
        ChannelProviderMetrics, DerivationPipelineMetrics, FrameQueueMetrics, L1RetrievalMetrics,
        L1TraversalMetrics, StepResult,
    },
};
use alloc::sync::Arc;
use core::fmt::Debug;

/// Composite metrics struct containing metrics for all stages.
#[derive(Clone)]
pub struct PipelineMetrics {
    pub(crate) derivation_pipeline_metrics: Arc<dyn DerivationPipelineMetrics + Send + Sync>,
    pub(crate) l1_traversal_metrics: Arc<dyn L1TraversalMetrics + Send + Sync>,
    pub(crate) l1_retrieval_metrics: Arc<dyn L1RetrievalMetrics + Send + Sync>,
    pub(crate) frame_queue_metrics: Arc<dyn FrameQueueMetrics + Send + Sync>,
    pub(crate) channel_provider_metrics: Arc<dyn ChannelProviderMetrics + Send + Sync>,
    // todo: add more metrics here for each stage
}

impl Debug for PipelineMetrics {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PipelineMetrics").finish()
    }
}

impl DerivationPipelineMetrics for PipelineMetrics {
    fn record_step_result(&self, result: &StepResult) {
        self.derivation_pipeline_metrics.record_step_result(result)
    }

    fn record_signal(&self, signal: &Signal) {
        self.derivation_pipeline_metrics.record_signal(signal)
    }
}

impl L1TraversalMetrics for PipelineMetrics {
    fn record_block_processed(&self, block_number: u64) {
        self.l1_traversal_metrics.record_block_processed(block_number)
    }

    fn record_system_config_update(&self) {
        self.l1_traversal_metrics.record_system_config_update()
    }

    fn record_reorg_detected(&self) {
        self.l1_traversal_metrics.record_reorg_detected()
    }

    fn record_holocene_activation(&self) {
        self.l1_traversal_metrics.record_holocene_activation()
    }
}

impl L1RetrievalMetrics for PipelineMetrics {
    fn record_data_fetch_attempt(&self, block_number: u64) {
        self.l1_retrieval_metrics.record_data_fetch_attempt(block_number)
    }

    fn record_data_fetch_success(&self, block_number: u64) {
        self.l1_retrieval_metrics.record_data_fetch_success(block_number)
    }

    fn record_data_fetch_failure(&self, block_number: u64, error: &PipelineErrorKind) {
        self.l1_retrieval_metrics.record_data_fetch_failure(block_number, error)
    }

    fn record_block_processed(&self, block_number: u64) {
        self.l1_retrieval_metrics.record_block_processed(block_number)
    }
}

impl FrameQueueMetrics for PipelineMetrics {
    fn record_frames_decoded(&self, count: usize) {
        self.frame_queue_metrics.record_frames_decoded(count)
    }

    fn record_frames_dropped(&self, count: usize) {
        self.frame_queue_metrics.record_frames_dropped(count)
    }

    fn record_frames_queued(&self, count: usize) {
        self.frame_queue_metrics.record_frames_queued(count)
    }

    fn record_load_frames_attempt(&self) {
        self.frame_queue_metrics.record_load_frames_attempt()
    }
}

impl ChannelProviderMetrics for PipelineMetrics {
    fn record_stage_transition(&self, from: &str, to: &str) {
        self.channel_provider_metrics.record_stage_transition(from, to)
    }

    fn record_data_item_provided(&self) {
        self.channel_provider_metrics.record_data_item_provided()
    }
}
