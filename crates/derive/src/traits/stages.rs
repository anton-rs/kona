//! This module contains common traits for stages within the derivation pipeline.

use crate::types::{BlockInfo, StageResult, SystemConfig};
use alloc::boxed::Box;
use async_trait::async_trait;

/// Describes the functionality fo a resettable stage within the derivation pipeline.
#[async_trait]
pub trait ResettableStage {
    /// Resets the derivation stage to its initial state.
    async fn reset(&mut self, base: BlockInfo, cfg: &SystemConfig) -> StageResult<()>;
}

/// Provides a method for accessing the pipeline's current L1 origin.
pub trait OriginProvider {
    /// Returns the optional L1 [BlockInfo] origin.
    fn origin(&self) -> Option<&BlockInfo>;
}

/// Provides a method for accessing a previous stage.
pub trait PreviousStage {
    /// The previous stage.
    type Previous: ResettableStage + PreviousStage;

    /// Returns the previous stage.
    fn previous(&self) -> Option<&Self::Previous>;
}
