use crate::{metrics::PipelineMetrics, pipeline::StepResult, traits::DerivationPipelineMetrics};
use alloc::sync::Arc;
use crate::pipeline::Signal;

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

    fn record_signal(&self, _signal: &Signal) {
        // No-op
    }
}
