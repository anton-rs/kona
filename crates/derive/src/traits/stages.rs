//! This module contains common traits for stages within the derivation pipeline.

use alloc::boxed::Box;
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::BlockInfo;

use crate::errors::PipelineResult;

/// Returns the previous stage in the pipeline.
pub trait PreviousStage {
    type Previous: PreviousStage + OriginProvider + OriginAdvancer + ResettableStage;

    /// Returns the previous stage in the pipeline.
    fn prev(&self) -> Option<&Self::Previous>;

    /// Returns the previous stage in the pipeline as a mutable reference.
    fn prev_mut(&mut self) -> Option<&mut Self::Previous>;
}

impl PreviousStage for () {
    type Previous = ();

    fn prev(&self) -> Option<&Self::Previous> {
        None
    }

    fn prev_mut(&mut self) -> Option<&mut Self::Previous> {
        None
    }
}

/// Describes the functionality fo a resettable stage within the derivation pipeline.
#[async_trait]
pub trait ResettableStage {
    /// Resets the derivation stage to its initial state.
    async fn reset(&mut self, base: BlockInfo, cfg: &SystemConfig) -> PipelineResult<()>;
}

#[async_trait]
impl ResettableStage for () {
    async fn reset(&mut self, _: BlockInfo, _: &SystemConfig) -> PipelineResult<()> {
        Ok(())
    }
}

/// Provides a method for accessing the pipeline's current L1 origin.
pub trait OriginProvider: PreviousStage {
    /// Returns the optional L1 [BlockInfo] origin.
    fn origin(&self) -> Option<BlockInfo> {
        self.prev().map(|p| p.origin()).flatten()
    }
}

impl OriginProvider for () {
    fn origin(&self) -> Option<BlockInfo> {
        None
    }
}

/// Defines a trait for advancing the L1 origin of the pipeline.
#[async_trait]
pub trait OriginAdvancer: PreviousStage + Send {
    /// Advances the internal state of the lowest stage to the next l1 origin.
    /// This method is the equivalent of the reference implementation `advance_l1_block`.
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        if let Some(prev) = self.prev_mut() {
            prev.advance_origin().await?;
        }
        Ok(())
    }
}

#[async_trait]
impl OriginAdvancer for () {
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}
