use crate::{
    metrics::PipelineMetrics,
    pipeline::{Signal, StepResult},
    traits::{AttributesQueueMetrics, DerivationPipelineMetrics},
};
use alloc::sync::Arc;

impl PipelineMetrics {
    /// No-op implementation for `PipelineMetrics`.
    pub fn no_op() -> Self {
        Self {
            derivation_pipeline_metrics: Arc::new(NoopDerivationPipelineMetrics),
            attributes_queue_metrics: Arc::new(NoopAttributesQueueMetrics),
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

/// No-op implementation of `DerivationPipelineMetrics`.
#[derive(Debug)]
struct NoopAttributesQueueMetrics;

impl AttributesQueueMetrics for NoopAttributesQueueMetrics {
    fn record_some_metric(&self) {
        // No-op
    }
}
