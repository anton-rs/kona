//! Testing utilities for the attributes queue stage.

use crate::{
    batch::SingleBatch,
    errors::{BuilderError, StageError, StageResult},
    stages::attributes_queue::{AttributesBuilder, AttributesProvider},
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_eips::BlockNumHash;
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OptimismPayloadAttributes;

/// A mock implementation of the [`AttributesBuilder`] for testing.
#[derive(Debug, Default)]
pub struct MockAttributesBuilder {
    /// The attributes to return.
    pub attributes: Vec<anyhow::Result<OptimismPayloadAttributes>>,
}

#[async_trait]
impl AttributesBuilder for MockAttributesBuilder {
    /// Prepares the [PayloadAttributes] for the next payload.
    async fn prepare_payload_attributes(
        &mut self,
        _l2_parent: L2BlockInfo,
        _epoch: BlockNumHash,
    ) -> Result<OptimismPayloadAttributes, BuilderError> {
        match self.attributes.pop() {
            Some(Ok(attrs)) => Ok(attrs),
            Some(Err(err)) => Err(BuilderError::Custom(err)),
            None => Err(BuilderError::Custom(anyhow::anyhow!("no attributes available"))),
        }
    }
}

/// A mock implementation of the [`BatchQueue`] stage for testing.
#[derive(Debug, Default)]
pub struct MockAttributesProvider {
    /// The origin of the L1 block.
    origin: Option<BlockInfo>,
    /// A list of batches to return.
    batches: Vec<StageResult<SingleBatch>>,
}

impl OriginProvider for MockAttributesProvider {
    fn origin(&self) -> Option<BlockInfo> {
        self.origin
    }
}

#[async_trait]
impl OriginAdvancer for MockAttributesProvider {
    async fn advance_origin(&mut self) -> StageResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ResettableStage for MockAttributesProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> StageResult<()> {
        Ok(())
    }
}

#[async_trait]
impl AttributesProvider for MockAttributesProvider {
    async fn next_batch(&mut self, _parent: L2BlockInfo) -> StageResult<SingleBatch> {
        self.batches.pop().ok_or(StageError::Eof)?
    }

    fn is_last_in_span(&self) -> bool {
        self.batches.is_empty()
    }
}

/// Creates a new [`MockAttributesProvider`] with the given origin and batches.
pub fn new_attributes_provider(
    origin: Option<BlockInfo>,
    batches: Vec<StageResult<SingleBatch>>,
) -> MockAttributesProvider {
    MockAttributesProvider { origin, batches }
}
