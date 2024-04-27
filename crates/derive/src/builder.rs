//! Contains a concrete implementation of the [DerivationPipeline].

use crate::{
    stages::NextAttributes,
    traits::{OriginAdvancer, ResettableStage},
    types::{
        BlockInfo, L2AttributesWithParent, L2BlockInfo, StageError, StageResult, SystemConfig,
    },
};
use alloc::{boxed::Box, collections::VecDeque};
use async_trait::async_trait;
use core::fmt::Debug;

/// Provides the [BlockInfo] and [SystemConfig] for the stack to reset the stages.
#[async_trait]
pub trait ResetProvider {
    /// Returns the current [BlockInfo] for the pipeline to reset.
    async fn block_info(&self) -> BlockInfo;

    /// Returns the current [SystemConfig] for the pipeline to reset.
    async fn system_config(&self) -> SystemConfig;
}

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

impl<
        S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
        R: ResetProvider + Send,
    > DerivationPipeline<S, R>
{
    /// Creates a new instance of the [DerivationPipeline].
    pub fn new(attributes: S, reset: R, cursor: L2BlockInfo) -> Self {
        Self { attributes, prepared: VecDeque::new(), reset, needs_reset: false, cursor }
    }

    /// Set the [L2BlockInfo] cursor to be used when pulling the next attributes.
    pub fn set_cursor(&mut self, cursor: L2BlockInfo) {
        self.cursor = cursor;
    }

    /// Returns the next [L2AttributesWithParent] from the pipeline.
    pub fn next_attributes(&mut self) -> Option<L2AttributesWithParent> {
        self.prepared.pop_front()
    }

    /// Flags the pipeline to reset on the next [DerivationPipeline::step] call.
    pub fn reset(&mut self) {
        self.needs_reset = true;
    }

    /// Resets the pipeline.
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

    /// Attempts to progress the pipeline.
    /// A [StageError::Eof] is returned if the pipeline is blocked by waiting for new L1 data.
    /// Any other error is critical and the derivation pipeline should be reset.
    /// An error is expected when the underlying source closes.
    /// When [DerivationPipeline::step] returns [Ok(())], it should be called again, to continue the
    /// derivation process.
    pub async fn step(&mut self) -> StageResult<()> {
        tracing::info!("DerivationPipeline::step");

        // Reset the pipeline if needed.
        if self.needs_reset {
            let block_info = self.reset.block_info().await;
            let system_config = self.reset.system_config().await;
            self.reset_pipe(block_info, &system_config).await?;
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
                self.attributes.advance_origin().await?;
            }
            // TODO: match on the EngineELSyncing error here and log
            Err(err) => {
                tracing::error!("attributes queue stage failed: {:?}", err);
                return Err(err);
            }
        }

        Ok(())
    }
}
