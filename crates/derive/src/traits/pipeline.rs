//! Defines the interface for the core derivation pipeline.

use super::OriginProvider;
use crate::errors::{PipelineErrorKind, PipelineResult};
use alloc::boxed::Box;
use async_trait::async_trait;
use core::iter::Iterator;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OptimismAttributesWithParent;

/// A pipeline error.
#[derive(Debug)]
pub enum StepResult {
    /// Attributes were successfully prepared.
    PreparedAttributes,
    /// Origin was advanced.
    AdvancedOrigin,
    /// Origin advance failed.
    OriginAdvanceErr(PipelineErrorKind),
    /// Step failed.
    StepFailed(PipelineErrorKind),
}

/// The type of pipeline reset.
#[derive(Debug)]
pub enum PipelineReset {
    /// A full pipeline reset.
    Full(BlockInfo, BlockInfo),
    /// A partial reset for holocene.
    Partial,
}

/// This trait defines the interface for interacting with the derivation pipeline.
#[async_trait]
pub trait Pipeline: OriginProvider + Iterator<Item = OptimismAttributesWithParent> {
    /// Peeks at the next [OptimismAttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&OptimismAttributesWithParent>;

    /// Resets the derivation pipeline.
    async fn reset(&mut self, reset: PipelineReset) -> PipelineResult<()>;

    /// Attempts to progress the pipeline.
    async fn step(&mut self, cursor: L2BlockInfo) -> StepResult;
}
