//! This module contains common traits for stages within the derivation pipeline.

use alloc::boxed::Box;
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::BlockInfo;

use crate::errors::PipelineResult;

/// Describes the functionality fo a resettable stage within the derivation pipeline.
#[async_trait]
pub trait ResettableStage {
    /// Resets the derivation stage to its initial state.
    async fn reset(&mut self, base: BlockInfo, cfg: &SystemConfig) -> PipelineResult<()>;
}

/// Provides a method for accessing the pipeline's current L1 origin.
pub trait OriginProvider {
    /// Returns the optional L1 [BlockInfo] origin.
    fn origin(&self) -> Option<BlockInfo>;
}

/// Defines a trait for advancing the L1 origin of the pipeline.
#[async_trait]
pub trait OriginAdvancer {
    /// Advances the internal state of the lowest stage to the next l1 origin.
    /// This method is the equivalent of the reference implementation `advance_l1_block`.
    async fn advance_origin(&mut self) -> PipelineResult<()>;
}
