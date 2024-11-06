//! Abstracts the derivation pipeline from the driver.

use alloc::{boxed::Box, sync::Arc};
use async_trait::async_trait;
use kona_derive::{errors::PipelineErrorKind, types::Signal};
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::L2BlockInfo;
use op_alloy_rpc_types_engine::OpAttributesWithParent;

/// Pipeline
///
/// A high-level abstraction for the driver's derivation pipeline.
#[async_trait]
pub trait Pipeline {
    /// Advance the pipeline to the target block.
    async fn produce_payload(
        &mut self,
        l2_safe_head: L2BlockInfo,
    ) -> Result<OpAttributesWithParent, PipelineErrorKind>;

    /// Signal the pipeline.
    async fn signal(&mut self, signal: Signal) -> Result<(), PipelineErrorKind>;

    /// Returns the Pipeline's rollup config.
    fn rollup_config(&self) -> Arc<RollupConfig>;
}
