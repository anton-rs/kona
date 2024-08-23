//! Contains traits for working with payload attributes and their providers.

use alloc::boxed::Box;
use async_trait::async_trait;
use kona_primitives::{L2AttributesWithParent, L2BlockInfo};

use crate::errors::StageResult;

/// [NextAttributes] defines the interface for pulling attributes from
/// the top level `AttributesQueue` stage of the pipeline.
#[async_trait]
pub trait NextAttributes {
    /// Returns the next [L2AttributesWithParent] from the current batch.
    async fn next_attributes(&mut self, parent: L2BlockInfo)
        -> StageResult<L2AttributesWithParent>;
}
