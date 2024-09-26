//! Contains the core derivation pipeline.

use super::{
    L2ChainProvider, NextAttributes, OriginAdvancer, OriginProvider, Pipeline, PipelineError,
    PipelineResult, ResettableStage, StepResult,
};
use crate::errors::PipelineErrorKind;
use alloc::{boxed::Box, collections::VecDeque, string::ToString, sync::Arc};
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OptimismAttributesWithParent;
use tracing::{error, trace, warn};

/// The derivation pipeline is responsible for deriving L2 inputs from L1 data.
#[derive(Debug)]
pub struct DerivationPipeline<S, P>
where
    S: NextAttributes + ResettableStage + OriginProvider + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
{
    /// A handle to the next attributes.
    pub attributes: S,
    /// Reset provider for the pipeline.
    /// A list of prepared [OptimismAttributesWithParent] to be used by the derivation pipeline
    /// consumer.
    pub prepared: VecDeque<OptimismAttributesWithParent>,
    /// The rollup config.
    pub rollup_config: Arc<RollupConfig>,
    /// The L2 Chain Provider used to fetch the system config on reset.
    pub l2_chain_provider: P,
}

impl<S, P> DerivationPipeline<S, P>
where
    S: NextAttributes + ResettableStage + OriginProvider + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
{
    /// Creates a new instance of the [DerivationPipeline].
    pub const fn new(
        attributes: S,
        rollup_config: Arc<RollupConfig>,
        l2_chain_provider: P,
    ) -> Self {
        Self { attributes, prepared: VecDeque::new(), rollup_config, l2_chain_provider }
    }
}

impl<S, P> OriginProvider for DerivationPipeline<S, P>
where
    S: NextAttributes + ResettableStage + OriginProvider + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.attributes.origin()
    }
}

impl<S, P> Iterator for DerivationPipeline<S, P>
where
    S: NextAttributes + ResettableStage + OriginProvider + OriginAdvancer + Debug + Send + Sync,
    P: L2ChainProvider + Send + Sync + Debug,
{
    type Item = OptimismAttributesWithParent;

    fn next(&mut self) -> Option<Self::Item> {
        self.prepared.pop_front()
    }
}

#[async_trait]
impl<S, P> Pipeline for DerivationPipeline<S, P>
where
    S: NextAttributes + ResettableStage + OriginProvider + OriginAdvancer + Debug + Send + Sync,
    P: L2ChainProvider + Send + Sync + Debug,
{
    /// Peeks at the next prepared [OptimismAttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&OptimismAttributesWithParent> {
        self.prepared.front()
    }

    /// Resets the pipeline by calling the [`ResettableStage::reset`] method.
    ///
    /// During a reset, each stage is recursively called from the top-level
    /// [crate::stages::AttributesQueue] to the bottom [crate::stages::L1Traversal]
    /// with a head-recursion pattern. This effectively clears the internal state
    /// of each stage in the pipeline from bottom on up.
    ///
    /// ### Parameters
    ///
    /// The `l2_block_info` is the new L2 cursor to step on. It is needed during
    /// reset to fetch the system config at that block height.
    ///
    /// The `l1_block_info` is the new L1 origin set in the [crate::stages::L1Traversal]
    /// stage.
    async fn reset(
        &mut self,
        l2_block_info: BlockInfo,
        l1_block_info: BlockInfo,
    ) -> PipelineResult<()> {
        let system_config = self
            .l2_chain_provider
            .system_config_by_number(l2_block_info.number, Arc::clone(&self.rollup_config))
            .await
            .map_err(|e| PipelineError::Provider(e.to_string()).temp())?;
        match self.attributes.reset(l1_block_info, &system_config).await {
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
        Ok(())
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
        match self.attributes.next_attributes(cursor).await {
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
        }
    }
}
