use crate::{
    errors::PipelineErrorKind,
    metrics::PipelineMetrics,
    pipeline::{Signal, StepResult},
    traits::{
        AttributesQueueMetrics, BatchQueueMetrics, BatchStreamMetrics, ChannelProviderMetrics,
        ChannelReaderMetrics, DerivationPipelineMetrics, FrameQueueMetrics, L1RetrievalMetrics,
        L1TraversalMetrics,
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
            channel_provider_metrics: Arc::new(NoopChannelProviderMetrics),
            channel_reader_metrics: Arc::new(NoopChannelReaderMetrics),
            batch_stream_metrics: Arc::new(NoopBatchStreamMetrics),
            batch_queue_metrics: Arc::new(NoopBatchQueueMetrics),
            atrirbutes_queue_metrics: Arc::new(NoopAttributesQueueMetrics),
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
}

#[derive(Debug)]
struct NoopChannelProviderMetrics;

impl ChannelProviderMetrics for NoopChannelProviderMetrics {
    fn record_stage_transition(&self, _from: &str, _to: &str) {
        // No-op
    }

    fn record_data_item_provided(&self) {
        // No-op
    }
}

#[derive(Debug)]
struct NoopChannelReaderMetrics;

impl ChannelReaderMetrics for NoopChannelReaderMetrics {
    fn record_batch_read(&self) {
        // No-op
    }

    fn record_channel_flushed(&self) {
        // No-op
    }
}

#[derive(Debug)]
struct NoopBatchStreamMetrics;

impl BatchStreamMetrics for NoopBatchStreamMetrics {
    fn record_batch_processed(&self) {
        // No-op
    }

    fn record_span_batch_accepted(&self) {
        // No-op
    }

    fn record_span_batch_dropped(&self) {
        // No-op
    }

    fn record_buffer_size(&self, _size: usize) {
        // No-op
    }
}

#[derive(Debug)]
struct NoopBatchQueueMetrics;

impl BatchQueueMetrics for NoopBatchQueueMetrics {
    fn record_batches_queued(&self, _count: usize) {
        // No-op
    }

    fn record_batch_dropped(&self) {
        // No-op
    }

    fn record_epoch_advanced(&self, _epoch: u64) {
        // No-op
    }
}

#[derive(Debug)]
struct NoopAttributesQueueMetrics;

impl AttributesQueueMetrics for NoopAttributesQueueMetrics {
    fn record_attributes_created(&self) {
        // No-op
    }

    fn record_batch_loaded(&self) {
        // No-op
    }

    fn record_attributes_creation_failure(&self) {
        // No-op
    }
}
