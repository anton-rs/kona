//! This module contains common traits for stages within the derivation pipeline.

use anyhow::Result;

use crate::types::{BlockInfo, SystemConfig};

/// Describes the functionality fo a resettable stage within the derivation pipeline.
pub trait ResettableStage {
    /// Resets the derivation stage to its initial state.
    fn reset(&mut self, base: BlockInfo, cfg: SystemConfig) -> Result<()>;
}
