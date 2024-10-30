use crate::{metrics::PipelineMetrics, pipeline::StepResult, traits::DerivationPipelineMetrics};
use alloc::sync::Arc;

impl PipelineMetrics {
    /// No-op implementation for `PipelineMetrics`.
    pub fn no_op() -> Self {
        Self {
            derivation_pipeline_metrics: Arc::new(NoopDerivationPipelineMetrics),
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
    fn inc_reset_signals(&self) {
        // No-op
    }
    fn inc_flush_channel_signals(&self) {
        // No-op
    }
}
