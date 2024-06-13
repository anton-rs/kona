//! Defines the interface for the core derivation pipeline.

use super::OriginProvider;
use alloc::boxed::Box;
use async_trait::async_trait;
use kona_primitives::{BlockInfo, L2AttributesWithParent, L2BlockInfo};

/// This trait defines the interface for interacting with the derivation pipeline.
#[async_trait]
pub trait Pipeline: OriginProvider {
    /// Returns the next [L2AttributesWithParent] from the pipeline.
    fn next_attributes(&mut self) -> Option<L2AttributesWithParent>;

    /// Resets the pipeline on the next [Pipeline::step] call.
    async fn reset(&mut self, origin: BlockInfo) -> anyhow::Result<()>;

    /// Attempts to progress the pipeline.
    async fn step(&mut self, cursor: &L2BlockInfo) -> anyhow::Result<()>;
}
