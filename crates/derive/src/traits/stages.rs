//! This module contains common traits for stages within the derivation pipeline.

use crate::types::{BlockInfo, StageResult, SystemConfig};
use alloc::boxed::Box;
use async_trait::async_trait;

/// Describes the functionality fo a resettable stage within the derivation pipeline.
#[async_trait]
pub trait ResettableStage {
    /// Resets the derivation stage to its initial state.
    async fn reset(&mut self, base: BlockInfo, cfg: SystemConfig) -> StageResult<()>;
}
