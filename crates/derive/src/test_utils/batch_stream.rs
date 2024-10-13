//! A mock implementation of the [`BatchStream`] stage for testing.
//!
//! [`BatchStream`]: crate::stages::BatchStream

use crate::{
    batch::Batch,
    errors::{PipelineError, PipelineResult},
    stages::BatchStreamProvider,
    traits::{FlushableStage, OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::BlockInfo;

/// A mock provider for the [`BatchStream`] stage.
///
/// [`BatchStream`]: crate::stages::BatchStream
#[derive(Debug, Default)]
pub struct TestBatchStreamProvider {
    /// The origin of the L1 block.
    pub origin: Option<BlockInfo>,
    /// A list of batches to return.
    pub batches: Vec<PipelineResult<Batch>>,
    /// Wether the reset method was called.
    pub reset: bool,
    /// Wether the provider was flushed.
    pub flushed: bool,
}

impl TestBatchStreamProvider {
    /// Creates a new [TestBatchStreamProvider] with the given origin and batches.
    pub fn new(batches: Vec<PipelineResult<Batch>>) -> Self {
        Self { origin: Some(BlockInfo::default()), batches, reset: false, flushed: false }
    }
}

impl OriginProvider for TestBatchStreamProvider {
    fn origin(&self) -> Option<BlockInfo> {
        self.origin
    }
}

#[async_trait]
impl FlushableStage for TestBatchStreamProvider {
    async fn flush_channel(&mut self) -> PipelineResult<()> {
        self.flushed = true;
        Ok(())
    }
}

#[async_trait]
impl BatchStreamProvider for TestBatchStreamProvider {
    fn flush(&mut self) {}

    async fn next_batch(&mut self) -> PipelineResult<Batch> {
        self.batches.pop().ok_or(PipelineError::Eof.temp())?
    }
}

#[async_trait]
impl OriginAdvancer for TestBatchStreamProvider {
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ResettableStage for TestBatchStreamProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> PipelineResult<()> {
        self.reset = true;
        Ok(())
    }
}
