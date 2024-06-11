//! Defines the interface for the core derivation pipeline.

use alloc::boxed::Box;
use async_trait::async_trait;
use kona_primitives::{BlockInfo, L2AttributesWithParent, L2BlockInfo};

/// This trait defines the interface for interacting with the derivation pipeline.
#[async_trait]
pub trait Pipeline {
    /// Resets the pipeline on the next [Pipeline::step] call.
    async fn reset(&mut self, origin: BlockInfo) -> anyhow::Result<()>;

    /// Sets the L1 Origin of the pipeline.
    fn set_origin(&mut self, origin: BlockInfo);

    /// Attempts to progress the pipeline.
    async fn step(&mut self) -> anyhow::Result<()>;

    /// Pops the next prepared [L2AttributesWithParent] from the pipeline.
    fn pop(&mut self) -> Option<L2AttributesWithParent>;

    /// Updates the L2 Safe Head cursor of the pipeline.
    /// This is used when fetching the next attributes.
    fn update_cursor(&mut self, cursor: L2BlockInfo);
}
