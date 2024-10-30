//! Metrics for the derivation pipeline.

mod noop;

use alloc::sync::Arc;
use core::fmt::Debug;

use crate::traits::{DerivationPipelineMetrics, StepResult};

/// Composite metrics struct containing metrics for all stages.
pub struct PipelineMetrics {
    pub(crate) derivation_pipeline_metrics: Arc<dyn DerivationPipelineMetrics + Send + Sync>,
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

    fn inc_reset_signals(&self) {
        self.derivation_pipeline_metrics.inc_reset_signals()
    }

    fn inc_flush_channel_signals(&self) {
        self.derivation_pipeline_metrics.inc_flush_channel_signals()
    }
}
