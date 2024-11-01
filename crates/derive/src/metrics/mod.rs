//! Metrics for the derivation pipeline.

mod noop;

use crate::{
    pipeline::Signal,
    traits::{AttributesQueueMetrics, DerivationPipelineMetrics, StepResult},
};
use alloc::sync::Arc;
use core::fmt::Debug;

/// Composite metrics struct containing metrics for all stages.
#[derive(Clone)]
pub struct PipelineMetrics {
    pub(crate) derivation_pipeline_metrics: Arc<dyn DerivationPipelineMetrics + Send + Sync>,
    pub(crate) attributes_queue_metrics: Arc<dyn AttributesQueueMetrics + Send + Sync>,
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

impl AttributesQueueMetrics for PipelineMetrics {
    fn record_some_metric(&self) {
        self.attributes_queue_metrics.record_some_metric()
    }
}
