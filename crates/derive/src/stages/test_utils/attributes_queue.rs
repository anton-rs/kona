//! Testing utilities for the attributes queue stage.

use crate::{
    batch::SingleBatch,
    errors::{BuilderError, PipelineError, PipelineErrorKind, PipelineResult},
    traits::{
        AttributesQueueBuilder, AttributesQueuePrior, OriginAdvancer, OriginProvider,
        ResettableStage,
    },
};
use alloc::{boxed::Box, string::ToString, vec::Vec};
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
impl AttributesQueueBuilder for MockAttributesBuilder {
    /// Prepares the [PayloadAttributes] for the next payload.
    async fn prepare_payload_attributes(
        &mut self,
        _l2_parent: L2BlockInfo,
        _epoch: BlockNumHash,
    ) -> PipelineResult<OptimismPayloadAttributes> {
        match self.attributes.pop() {
            Some(Ok(attrs)) => Ok(attrs),
            Some(Err(err)) => {
                Err(PipelineErrorKind::Temporary(BuilderError::Custom(err.to_string()).into()))
            }
            None => Err(PipelineErrorKind::Critical(BuilderError::AttributesUnavailable.into())),
        }
    }
}

/// A mock implementation of the [`BatchQueue`] stage for testing.
#[derive(Debug, Default)]
pub struct MockAttributesProvider {
    /// The origin of the L1 block.
    origin: Option<BlockInfo>,
    /// A list of batches to return.
    batches: Vec<PipelineResult<SingleBatch>>,
}

impl OriginProvider for MockAttributesProvider {
    fn origin(&self) -> Option<BlockInfo> {
        self.origin
    }
}

#[async_trait]
impl OriginAdvancer for MockAttributesProvider {
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ResettableStage for MockAttributesProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait]
impl AttributesQueuePrior for MockAttributesProvider {
    async fn next_batch(&mut self, _parent: L2BlockInfo) -> PipelineResult<SingleBatch> {
        self.batches.pop().ok_or(PipelineError::Eof.temp())?
    }

    fn is_last_in_span(&self) -> bool {
        self.batches.is_empty()
    }
}

/// Creates a new [`MockAttributesProvider`] with the given origin and batches.
pub const fn new_attributes_provider(
    origin: Option<BlockInfo>,
    batches: Vec<PipelineResult<SingleBatch>>,
) -> MockAttributesProvider {
    MockAttributesProvider { origin, batches }
}
