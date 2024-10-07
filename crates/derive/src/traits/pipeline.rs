//! Defines the interface for the core derivation pipeline.

use super::OriginProvider;
use crate::errors::{PipelineErrorKind, PipelineResult};
use alloc::boxed::Box;
use async_trait::async_trait;
use core::iter::Iterator;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OptimismAttributesWithParent;

/// A pipeline error.
#[derive(Debug, PartialEq, Eq)]
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

/// A signal to send to the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Signal {
    /// Reset the pipeline.
    Reset {
        /// The L2 safe head to reset to.
        l2_safe_head: L2BlockInfo,
        /// The L1 origin to reset to.
        l1_origin: BlockInfo,
    },
    /// Flush the currently active channel.
    FlushChannel,
}

/// This trait defines the interface for interacting with the derivation pipeline.
#[async_trait]
pub trait Pipeline: OriginProvider + Iterator<Item = OptimismAttributesWithParent> {
    /// Peeks at the next [OptimismAttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&OptimismAttributesWithParent>;

    /// Resets the pipeline on the next [Pipeline::step] call.
    async fn signal(&mut self, signal: Signal) -> PipelineResult<()>;

    /// Attempts to progress the pipeline.
    async fn step(&mut self, cursor: L2BlockInfo) -> StepResult;
}
