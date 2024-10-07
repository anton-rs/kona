//! Test Utilities for `kona-derive`.
//!
//! This includes top-level [crate::pipeline::DerivationPipeline]
//! test utilities as well as individual stage test utilities.

use alloc::{boxed::Box, sync::Arc};
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OptimismAttributesWithParent;

// Re-export these types used internally to the test pipeline.
pub use crate::{
    batch::SingleBatch,
    errors::PipelineError,
    pipeline::{DerivationPipeline, PipelineBuilder, PipelineResult},
    stages::{
        test_utils::MockAttributesBuilder, AttributesProvider, AttributesQueue, BatchQueue,
        BatchStream, ChannelBank, ChannelReader, FrameQueue, L1Retrieval, L1Traversal,
    },
    traits::{
        test_utils::TestDAP, FlushableStage, NextAttributes, OriginAdvancer, OriginProvider,
        ResettableStage,
    },
};
pub use kona_providers::test_utils::{TestChainProvider, TestL2ChainProvider};

/// A fully custom [NextAttributes].
#[derive(Default, Debug, Clone)]
pub struct TestNextAttributes {
    /// The next [OptimismAttributesWithParent] to return.
    pub next_attributes: Option<OptimismAttributesWithParent>,
}

#[async_trait::async_trait]
impl FlushableStage for TestNextAttributes {
    /// Flushes the stage.
    async fn flush_channel(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl ResettableStage for TestNextAttributes {
    /// Resets the derivation stage to its initial state.
    async fn reset(&mut self, _: BlockInfo, _: &SystemConfig) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OriginProvider for TestNextAttributes {
    /// Returns the current origin.
    fn origin(&self) -> Option<BlockInfo> {
        Some(BlockInfo::default())
    }
}

#[async_trait::async_trait]
impl OriginAdvancer for TestNextAttributes {
    /// Advances the origin to the given block.
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl NextAttributes for TestNextAttributes {
    /// Returns the next valid attributes.
    async fn next_attributes(
        &mut self,
        _: L2BlockInfo,
    ) -> PipelineResult<OptimismAttributesWithParent> {
        self.next_attributes.take().ok_or(PipelineError::Eof.temp())
    }
}

/// An [L1Traversal] using test providers and sources.
pub type TestL1Traversal = L1Traversal<TestChainProvider>;

/// An [L1Retrieval] stage using test providers and sources.
pub type TestL1Retrieval = L1Retrieval<TestDAP, TestL1Traversal>;

/// A [FrameQueue] using test providers and sources.
pub type TestFrameQueue = FrameQueue<TestL1Retrieval>;

/// A [ChannelBank] using test providers and sources.
pub type TestChannelBank = ChannelBank<TestFrameQueue>;

/// A [ChannelReader] using test providers and sources.
pub type TestChannelReader = ChannelReader<TestChannelBank>;

/// A [BatchStream] using test providers and sources.
pub type TestBatchStream = BatchStream<TestChannelReader, TestL2ChainProvider>;

/// A [BatchQueue] using test providers and sources.
pub type TestBatchQueue = BatchQueue<TestBatchStream, TestL2ChainProvider>;

/// An [AttributesQueue] using test providers and sources.
pub type TestAttributesQueue = AttributesQueue<TestBatchQueue, MockAttributesBuilder>;

/// A [DerivationPipeline] using test providers and sources.
pub type TestPipeline = DerivationPipeline<TestAttributesQueue, TestL2ChainProvider>;

/// Constructs a [DerivationPipeline] using test providers and sources.
pub fn new_test_pipeline() -> TestPipeline {
    PipelineBuilder::new()
        .rollup_config(Arc::new(RollupConfig::default()))
        .origin(BlockInfo::default())
        .dap_source(TestDAP::default())
        .builder(MockAttributesBuilder::default())
        .chain_provider(TestChainProvider::default())
        .l2_chain_provider(TestL2ChainProvider::default())
        .build()
}
