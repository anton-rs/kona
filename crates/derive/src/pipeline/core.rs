//! Contains the core derivation pipeline.

use super::{
    NextAttributes, OriginAdvancer, OriginProvider, Pipeline, PipelineError, PipelineResult,
    StepResult,
};
use crate::{
    errors::PipelineErrorKind,
    traits::{
        ActivationSignal, DerivationPipelineMetrics, L2ChainProvider, ResetSignal, Signal,
        SignalReceiver,
    },
};
use alloc::{boxed::Box, collections::VecDeque, string::ToString, sync::Arc};
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OpAttributesWithParent;
use tracing::{error, trace, warn};

/// The derivation pipeline is responsible for deriving L2 inputs from L1 data.
#[derive(Debug)]
pub struct DerivationPipeline<S, P, M>
where
    S: NextAttributes + SignalReceiver + OriginProvider + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
    M: DerivationPipelineMetrics + Send + Sync,
{
    /// A handle to the next attributes.
    pub attributes: S,
    /// Reset provider for the pipeline.
    /// A list of prepared [OpAttributesWithParent] to be used by the derivation pipeline
    /// consumer.
    pub prepared: VecDeque<OpAttributesWithParent>,
    /// The rollup config.
    pub rollup_config: Arc<RollupConfig>,
    /// The L2 Chain Provider used to fetch the system config on reset.
    pub l2_chain_provider: P,
    /// Metrics collector.
    pub metrics: M,
}

impl<S, P, M> DerivationPipeline<S, P, M>
where
    S: NextAttributes + SignalReceiver + OriginProvider + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
    M: DerivationPipelineMetrics + Send + Sync,
{
    /// Creates a new instance of the [DerivationPipeline].
    pub const fn new(
        attributes: S,
        rollup_config: Arc<RollupConfig>,
        l2_chain_provider: P,
        metrics: M,
    ) -> Self {
        Self { attributes, prepared: VecDeque::new(), rollup_config, l2_chain_provider, metrics }
    }
}

impl<S, P, M> OriginProvider for DerivationPipeline<S, P, M>
where
    S: NextAttributes + SignalReceiver + OriginProvider + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
    M: DerivationPipelineMetrics + Send + Sync,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.attributes.origin()
    }
}

impl<S, P, M> Iterator for DerivationPipeline<S, P, M>
where
    S: NextAttributes + SignalReceiver + OriginProvider + OriginAdvancer + Debug + Send + Sync,
    P: L2ChainProvider + Send + Sync + Debug,
    M: DerivationPipelineMetrics + Send + Sync,
{
    type Item = OpAttributesWithParent;

    fn next(&mut self) -> Option<Self::Item> {
        self.prepared.pop_front()
    }
}

#[async_trait]
impl<S, P, M> SignalReceiver for DerivationPipeline<S, P, M>
where
    S: NextAttributes + SignalReceiver + OriginProvider + OriginAdvancer + Debug + Send + Sync,
    P: L2ChainProvider + Send + Sync + Debug,
    M: DerivationPipelineMetrics + Send + Sync,
{
    /// Signals the pipeline by calling the [`SignalReceiver::signal`] method.
    ///
    /// During a [`Signal::Reset`], each stage is recursively called from the top-level
    /// [crate::stages::AttributesQueue] to the bottom [crate::stages::L1Traversal]
    /// with a head-recursion pattern. This effectively clears the internal state
    /// of each stage in the pipeline from bottom on up.
    ///
    /// [`Signal::Activation`] does a similar thing to the reset, with different
    /// holocene-specific reset rules.
    ///
    /// ### Parameters
    ///
    /// The `signal` is contains the signal variant with any necessary parameters.
    async fn signal(&mut self, signal: Signal) -> PipelineResult<()> {
        match signal {
            s @ Signal::Reset(ResetSignal { l2_safe_head, .. }) |
            s @ Signal::Activation(ActivationSignal { l2_safe_head, .. }) => {
                self.metrics.inc_reset_signals();

                let system_config = self
                    .l2_chain_provider
                    .system_config_by_number(
                        l2_safe_head.block_info.number,
                        Arc::clone(&self.rollup_config),
                    )
                    .await
                    .map_err(|e| PipelineError::Provider(e.to_string()).temp())?;
                s.with_system_config(system_config);
                match self.attributes.signal(s).await {
                    Ok(()) => trace!(target: "pipeline", "Stages reset"),
                    Err(err) => {
                        if let PipelineErrorKind::Temporary(PipelineError::Eof) = err {
                            trace!(target: "pipeline", "Stages reset with EOF");
                        } else {
                            error!(target: "pipeline", "Stage reset errored: {:?}", err);
                            return Err(err);
                        }
                    }
                }
            }
            Signal::FlushChannel => {
                self.metrics.inc_flush_channel_signals();

                self.attributes.signal(signal).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<S, P, M> Pipeline for DerivationPipeline<S, P, M>
where
    S: NextAttributes + SignalReceiver + OriginProvider + OriginAdvancer + Debug + Send + Sync,
    P: L2ChainProvider + Send + Sync + Debug,
    M: DerivationPipelineMetrics + Send + Sync,
{
    /// Peeks at the next prepared [OpAttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&OpAttributesWithParent> {
        self.prepared.front()
    }

    /// Attempts to progress the pipeline.
    ///
    /// ## Returns
    ///
    /// A [PipelineError::Eof] is returned if the pipeline is blocked by waiting for new L1 data.
    /// Any other error is critical and the derivation pipeline should be reset.
    /// An error is expected when the underlying source closes.
    ///
    /// When [DerivationPipeline::step] returns [Ok(())], it should be called again, to continue the
    /// derivation process.
    ///
    /// [PipelineError]: crate::errors::PipelineError
    async fn step(&mut self, cursor: L2BlockInfo) -> StepResult {
        let result = match self.attributes.next_attributes(cursor).await {
            Ok(a) => {
                trace!(target: "pipeline", "Prepared L2 attributes: {:?}", a);
                self.prepared.push_back(a);
                StepResult::PreparedAttributes
            }
            Err(err) => match err {
                PipelineErrorKind::Temporary(PipelineError::Eof) => {
                    trace!(target: "pipeline", "Pipeline advancing origin");
                    if let Err(e) = self.attributes.advance_origin().await {
                        return StepResult::OriginAdvanceErr(e);
                    }
                    StepResult::AdvancedOrigin
                }
                _ => {
                    warn!(target: "pipeline", "Attributes queue step failed: {:?}", err);
                    StepResult::StepFailed(err)
                }
            },
        };

        self.metrics.record_step_result(&result);

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        metrics::PipelineMetrics,
        pipeline::{DerivationPipeline, PipelineError, StepResult},
        test_utils::{TestL2ChainProvider, *},
        traits::{ActivationSignal, Pipeline, ResetSignal, Signal, SignalReceiver},
    };
    use alloc::{string::ToString, sync::Arc};
    use alloy_rpc_types_engine::PayloadAttributes;
    use op_alloy_genesis::{RollupConfig, SystemConfig};
    use op_alloy_protocol::L2BlockInfo;
    use op_alloy_rpc_types_engine::{OpAttributesWithParent, OpPayloadAttributes};

    fn default_test_payload_attributes() -> OpAttributesWithParent {
        OpAttributesWithParent {
            attributes: OpPayloadAttributes {
                payload_attributes: PayloadAttributes {
                    timestamp: 0,
                    prev_randao: Default::default(),
                    suggested_fee_recipient: Default::default(),
                    withdrawals: None,
                    parent_beacon_block_root: None,
                },
                transactions: None,
                no_tx_pool: None,
                gas_limit: None,
                eip_1559_params: None,
            },
            parent: Default::default(),
            is_last_in_span: false,
        }
    }

    #[test]
    fn test_pipeline_next_attributes_empty() {
        let mut pipeline = new_test_pipeline();
        let result = pipeline.next();
        assert_eq!(result, None);
    }

    #[test]
    fn test_pipeline_next_attributes_with_peek() {
        let mut pipeline = new_test_pipeline();
        let expected = default_test_payload_attributes();
        pipeline.prepared.push_back(expected.clone());

        let result = pipeline.peek();
        assert_eq!(result, Some(&expected));

        let result = pipeline.next();
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn test_derivation_pipeline_missing_block() {
        let mut pipeline = new_test_pipeline();
        let cursor = L2BlockInfo::default();
        let result = pipeline.step(cursor).await;
        assert_eq!(
            result,
            StepResult::OriginAdvanceErr(
                PipelineError::Provider("Block not found".to_string()).temp()
            )
        );
    }

    #[tokio::test]
    async fn test_derivation_pipeline_prepared_attributes() {
        let rollup_config = Arc::new(RollupConfig::default());
        let l2_chain_provider = TestL2ChainProvider::default();
        let expected = default_test_payload_attributes();
        let attributes = TestNextAttributes { next_attributes: Some(expected) };
        let metrics = PipelineMetrics::no_op();
        let mut pipeline =
            DerivationPipeline::new(attributes, rollup_config, l2_chain_provider, metrics);

        // Step on the pipeline and expect the result.
        let cursor = L2BlockInfo::default();
        let result = pipeline.step(cursor).await;
        assert_eq!(result, StepResult::PreparedAttributes);
    }

    #[tokio::test]
    async fn test_derivation_pipeline_advance_origin() {
        let rollup_config = Arc::new(RollupConfig::default());
        let l2_chain_provider = TestL2ChainProvider::default();
        let attributes = TestNextAttributes::default();
        let metrics = PipelineMetrics::no_op();
        let mut pipeline =
            DerivationPipeline::new(attributes, rollup_config, l2_chain_provider, metrics);

        // Step on the pipeline and expect the result.
        let cursor = L2BlockInfo::default();
        let result = pipeline.step(cursor).await;
        assert_eq!(result, StepResult::AdvancedOrigin);
    }

    #[tokio::test]
    async fn test_derivation_pipeline_signal_activation() {
        let rollup_config = Arc::new(RollupConfig::default());
        let mut l2_chain_provider = TestL2ChainProvider::default();
        l2_chain_provider.system_configs.insert(0, SystemConfig::default());
        let attributes = TestNextAttributes::default();
        let metrics = PipelineMetrics::no_op();
        let mut pipeline =
            DerivationPipeline::new(attributes, rollup_config, l2_chain_provider, metrics);

        // Signal the pipeline to reset.
        let result = pipeline.signal(ActivationSignal::default().signal()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_derivation_pipeline_flush_channel() {
        let rollup_config = Arc::new(RollupConfig::default());
        let l2_chain_provider = TestL2ChainProvider::default();
        let attributes = TestNextAttributes::default();
        let metrics = PipelineMetrics::no_op();
        let mut pipeline =
            DerivationPipeline::new(attributes, rollup_config, l2_chain_provider, metrics);

        // Signal the pipeline to reset.
        let result = pipeline.signal(Signal::FlushChannel).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_derivation_pipeline_signal_reset_missing_sys_config() {
        let rollup_config = Arc::new(RollupConfig::default());
        let l2_chain_provider = TestL2ChainProvider::default();
        let attributes = TestNextAttributes::default();
        let metrics = PipelineMetrics::no_op();
        let mut pipeline =
            DerivationPipeline::new(attributes, rollup_config, l2_chain_provider, metrics);

        // Signal the pipeline to reset.
        let result = pipeline.signal(ResetSignal::default().signal()).await.unwrap_err();
        assert_eq!(result, PipelineError::Provider("System config not found".to_string()).temp());
    }

    #[tokio::test]
    async fn test_derivation_pipeline_signal_reset_ok() {
        let rollup_config = Arc::new(RollupConfig::default());
        let mut l2_chain_provider = TestL2ChainProvider::default();
        l2_chain_provider.system_configs.insert(0, SystemConfig::default());
        let attributes = TestNextAttributes::default();
        let metrics = PipelineMetrics::no_op();
        let mut pipeline =
            DerivationPipeline::new(attributes, rollup_config, l2_chain_provider, metrics);

        // Signal the pipeline to reset.
        let result = pipeline.signal(ResetSignal::default().signal()).await;
        assert!(result.is_ok());
    }
}
