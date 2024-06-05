//! Contains the core derivation pipeline.

use super::{
    NextAttributes, OriginAdvancer, Pipeline, ResetProvider, ResettableStage, StageError,
    StageResult,
};
use alloc::{boxed::Box, collections::VecDeque};
use async_trait::async_trait;
use core::fmt::Debug;
use kona_primitives::{BlockInfo, L2AttributesWithParent, L2BlockInfo, SystemConfig};

/// The derivation pipeline is responsible for deriving L2 inputs from L1 data.
#[derive(Debug)]
pub struct DerivationPipeline<
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
> {
    /// A handle to the next attributes.
    pub attributes: S,
    /// Reset provider for the pipeline.
    pub reset: R,
    /// A list of prepared [L2AttributesWithParent] to be used by the derivation pipeline consumer.
    pub prepared: VecDeque<L2AttributesWithParent>,
    /// A flag to tell the pipeline to reset.
    pub needs_reset: bool,
    /// A cursor for the [L2BlockInfo] parent to be used when pulling the next attributes.
    pub cursor: L2BlockInfo,
}

#[async_trait]
impl<S, R> Pipeline for DerivationPipeline<S, R>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
{
    fn reset(&mut self) {
        self.needs_reset = true;
    }

    /// Pops the next prepared [L2AttributesWithParent] from the pipeline.
    fn pop(&mut self) -> Option<L2AttributesWithParent> {
        self.prepared.pop_front()
    }

    /// Updates the L2 Safe Head cursor of the pipeline.
    /// The cursor is used to fetch the next attributes.
    fn update_cursor(&mut self, cursor: L2BlockInfo) {
        self.cursor = cursor;
    }

    /// Attempts to progress the pipeline.
    /// A [StageError::Eof] is returned if the pipeline is blocked by waiting for new L1 data.
    /// Any other error is critical and the derivation pipeline should be reset.
    /// An error is expected when the underlying source closes.
    /// When [DerivationPipeline::step] returns [Ok(())], it should be called again, to continue the
    /// derivation process.
    async fn step(&mut self) -> anyhow::Result<()> {
        tracing::info!("DerivationPipeline::step");

        // Reset the pipeline if needed.
        if self.needs_reset {
            let block_info = self.reset.block_info().await;
            let system_config = self.reset.system_config().await;
            self.reset_pipe(block_info, &system_config).await.map_err(|e| anyhow::anyhow!(e))?;
            self.needs_reset = false;
        }

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
                tracing::error!("attributes queue stage failed: {:?}", err);
                return Err(anyhow::anyhow!(err));
            }
        }

        Ok(())
    }
}

impl<S, R> DerivationPipeline<S, R>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
{
    /// Creates a new instance of the [DerivationPipeline].
    pub fn new(attributes: S, reset: R, cursor: L2BlockInfo) -> Self {
        Self { attributes, prepared: VecDeque::new(), reset, needs_reset: false, cursor }
    }

    /// Internal helper to reset the pipeline.
    async fn reset_pipe(&mut self, bi: BlockInfo, sc: &SystemConfig) -> StageResult<()> {
        match self.attributes.reset(bi, sc).await {
            Ok(()) => {
                tracing::info!("Stages reset");
            }
            Err(StageError::Eof) => {
                tracing::info!("Stages reset with EOF");
            }
            Err(err) => {
                tracing::error!("Stages reset failed: {:?}", err);
                return Err(err);
            }
        }
        Ok(())
    }
}
