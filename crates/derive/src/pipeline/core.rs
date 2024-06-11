//! Contains the core derivation pipeline.

use super::{
    L2ChainProvider, NextAttributes, OriginAdvancer, Pipeline, ResettableStage, StageError,
};
use alloc::{boxed::Box, collections::VecDeque, sync::Arc};
use async_trait::async_trait;
use core::fmt::Debug;
use kona_primitives::{BlockInfo, L2AttributesWithParent, L2BlockInfo, RollupConfig};

/// The derivation pipeline is responsible for deriving L2 inputs from L1 data.
#[derive(Debug)]
pub struct DerivationPipeline<S, P>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
{
    /// A handle to the next attributes.
    pub attributes: S,
    /// Reset provider for the pipeline.
    /// A list of prepared [L2AttributesWithParent] to be used by the derivation pipeline consumer.
    pub prepared: VecDeque<L2AttributesWithParent>,
    /// A cursor for the [L2BlockInfo] parent to be used when pulling the next attributes.
    pub cursor: L2BlockInfo,
    /// L1 Origin Tip
    pub tip: BlockInfo,
    /// The rollup config.
    pub rollup_config: Arc<RollupConfig>,
    /// The L2 Chain Provider used to fetch the system config on reset.
    pub l2_chain_provider: P,
}

impl<S, P> DerivationPipeline<S, P>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
{
    /// Creates a new instance of the [DerivationPipeline].
    pub fn new(
        attributes: S,
        tip: BlockInfo,
        cursor: L2BlockInfo,
        rollup_config: Arc<RollupConfig>,
        l2_chain_provider: P,
    ) -> Self {
        Self {
            attributes,
            prepared: VecDeque::new(),
            rollup_config,
            tip,
            cursor,
            l2_chain_provider,
        }
    }
}

#[async_trait]
impl<S, P> Pipeline for DerivationPipeline<S, P>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    P: L2ChainProvider + Send + Sync + Debug,
{
    /// Pops the next prepared [L2AttributesWithParent] from the pipeline.
    fn pop(&mut self) -> Option<L2AttributesWithParent> {
        self.prepared.pop_front()
    }

    /// Updates the L2 Safe Head cursor of the pipeline.
    /// The cursor is used to fetch the next attributes.
    fn update_cursor(&mut self, cursor: L2BlockInfo) {
        self.cursor = cursor;
    }

    /// Sets the L1 Origin of the pipeline.
    fn set_origin(&mut self, origin: BlockInfo) {
        self.tip = origin;
    }

    /// Resets the pipelien by calling the [`ResettableStage::reset`] method.
    /// This will bubble down the stages all the way to the `L1Traversal` stage.
    async fn reset(&mut self, block_info: BlockInfo) -> anyhow::Result<()> {
        self.tip = block_info;
        let system_config = self
            .l2_chain_provider
            .system_config_by_number(self.tip.number, Arc::clone(&self.rollup_config))
            .await?;
        match self.attributes.reset(self.tip, &system_config).await {
            Ok(()) => tracing::info!("Stages reset"),
            Err(StageError::Eof) => tracing::info!("Stages reset with EOF"),
            Err(err) => {
                tracing::error!("Stages reset failed: {:?}", err);
                anyhow::bail!(err);
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
    async fn step(&mut self) -> anyhow::Result<()> {
        match self.attributes.next_attributes(self.cursor).await {
            Ok(a) => {
                tracing::info!("attributes queue stage step returned l2 attributes");
                tracing::info!("prepared L2 attributes: {:?}", a);
                self.prepared.push_back(a);
                return Ok(());
            }
            Err(StageError::Eof) => {
                tracing::info!("attributes queue stage complete");
                self.attributes.advance_origin().await.map_err(|e| anyhow::anyhow!(e))?;
            }
            // TODO: match on the EngineELSyncing error here and log
            Err(err) => {
                tracing::error!("attributes queue step failed: {:?}", err);
                return Err(anyhow::anyhow!(err));
            }
        }
        Ok(())
    }
}
