//! Defines the interface for the core derivation pipeline.

use super::OriginProvider;
use crate::types::StageError;
use alloc::boxed::Box;
use async_trait::async_trait;
use core::iter::Iterator;
use kona_primitives::{BlockInfo, L2AttributesWithParent, L2BlockInfo};

/// A pipeline error.
#[derive(Debug)]
pub enum StepResult {
    /// Attributes were successfully prepared.
    PreparedAttributes,
    /// Origin was advanced.
    AdvancedOrigin,
    /// Origin advance failed.
    OriginAdvanceErr(StageError),
    /// Step failed.
    StepFailed(StageError),
}

/// This trait defines the interface for interacting with the derivation pipeline.
#[async_trait]
pub trait Pipeline: OriginProvider + Iterator<Item = L2AttributesWithParent> {
    /// Peeks at the next [L2AttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&L2AttributesWithParent>;

    /// Resets the pipeline on the next [Pipeline::step] call.
    async fn reset(&mut self, origin: BlockInfo) -> anyhow::Result<()>;

    /// Attempts to progress the pipeline.
    async fn step(&mut self, cursor: L2BlockInfo) -> StepResult;
}
