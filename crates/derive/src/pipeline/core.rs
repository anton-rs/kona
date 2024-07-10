//! Contains the core derivation pipeline.

use super::{
    L2ChainProvider, NextAttributes, OriginAdvancer, OriginProvider, Pipeline, ResettableStage,
    StageError, StepResult,
};
use alloc::{boxed::Box, collections::VecDeque, sync::Arc};
use anyhow::bail;
use async_trait::async_trait;
use core::fmt::Debug;
use kona_primitives::{BlockInfo, L2AttributesWithParent, L2BlockInfo, RollupConfig};
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
    /// A list of prepared [L2AttributesWithParent] to be used by the derivation pipeline consumer.
    pub prepared: VecDeque<L2AttributesWithParent>,
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
    pub fn new(attributes: S, rollup_config: Arc<RollupConfig>, l2_chain_provider: P) -> Self {
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
    type Item = L2AttributesWithParent;

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
    /// Peeks at the next prepared [L2AttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&L2AttributesWithParent> {
        self.prepared.front()
    }

    /// Resets the pipelien by calling the [`ResettableStage::reset`] method.
    /// This will bubble down the stages all the way to the `L1Traversal` stage.
    async fn reset(&mut self, block_info: BlockInfo) -> anyhow::Result<()> {
        let system_config = self
            .l2_chain_provider
            .system_config_by_number(block_info.number, Arc::clone(&self.rollup_config))
            .await?;
        match self.attributes.reset(block_info, &system_config).await {
            Ok(()) => trace!(target: "pipeline", "Stages reset"),
            Err(StageError::Eof) => trace!(target: "pipeline", "Stages reset with EOF"),
            Err(err) => {
                error!(target: "pipeline", "Stage reset errored: {:?}", err);
                bail!(err);
            }
        }
        Ok(())
    }

    /// Attempts to progress the pipeline.
    /// A [StageError::Eof] is returned if the pipeline is blocked by waiting for new L1 data.
    /// Any other error is critical and the derivation pipeline should be reset.
    /// An error is expected when the underlying source closes.
    /// When [DerivationPipeline::step] returns [Ok(())], it should be called again, to continue the
    /// derivation process.
    async fn step(&mut self, cursor: L2BlockInfo) -> StepResult {
        match self.attributes.next_attributes(cursor).await {
            Ok(a) => {
                trace!(target: "pipeline", "Prepared L2 attributes: {:?}", a);
                self.prepared.push_back(a);
                StepResult::PreparedAttributes
            }
            Err(StageError::Eof) => {
                trace!(target: "pipeline", "Pipeline advancing origin");
                if let Err(e) = self.attributes.advance_origin().await {
                    return StepResult::OriginAdvanceErr(e);
                }
                StepResult::AdvancedOrigin
            }
            Err(err) => {
                warn!(target: "pipeline", "Attributes queue step failed: {:?}", err);
                StepResult::StepFailed(err)
            }
        }
    }
}
