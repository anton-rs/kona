//! Defines the interface for the core derivation pipeline.

use super::OriginProvider;
use crate::types::StepResult;
use alloc::boxed::Box;
use async_trait::async_trait;
use core::iter::Iterator;
use op_alloy_protocol::L2BlockInfo;
use op_alloy_rpc_types_engine::OpAttributesWithParent;

/// This trait defines the interface for interacting with the derivation pipeline.
#[async_trait]
pub trait Pipeline: OriginProvider + Iterator<Item = OpAttributesWithParent> {
    /// Peeks at the next [OpAttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&OpAttributesWithParent>;

    /// Attempts to progress the pipeline.
    async fn step(&mut self, cursor: L2BlockInfo) -> StepResult;
}
