//! Contains online pipeline types.

use crate::{
    online::{
        AlloyChainProvider, AlloyL2ChainProvider, OnlineBeaconClient, OnlineBlobProvider,
        SimpleSlotDerivation,
    },
    pipeline::{DerivationPipeline, PipelineBuilder},
    sources::EthereumDataSource,
    stages::{
        AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue, L1Retrieval,
        L1Traversal, StatefulAttributesBuilder,
    },
    types::RollupConfig,
};
use alloc::sync::Arc;

/// An online derivation pipeline.
pub type OnlinePipeline =
    DerivationPipeline<OnlineAttributesQueue<OnlineDataProvider>, AlloyL2ChainProvider>;

/// An `online` Ethereum data source.
pub type OnlineDataProvider = EthereumDataSource<
    AlloyChainProvider,
    OnlineBlobProvider<OnlineBeaconClient, SimpleSlotDerivation>,
>;

/// An `online` payload attributes builder for the `AttributesQueue` stage of the derivation
/// pipeline.
pub type OnlineAttributesBuilder =
    StatefulAttributesBuilder<AlloyChainProvider, AlloyL2ChainProvider>;

/// An `online` attributes queue for the derivation pipeline.
pub type OnlineAttributesQueue<DAP> = AttributesQueue<
    BatchQueue<
        ChannelReader<ChannelBank<FrameQueue<L1Retrieval<DAP, L1Traversal<AlloyChainProvider>>>>>,
        AlloyL2ChainProvider,
    >,
    OnlineAttributesBuilder,
>;

/// Creates a new online [DerivationPipeline] from the given inputs.
/// Internally, this uses the [PipelineBuilder] to construct the pipeline.
pub fn new_online_pipeline(
    rollup_config: Arc<RollupConfig>,
    chain_provider: AlloyChainProvider,
    dap_source: OnlineDataProvider,
    l2_chain_provider: AlloyL2ChainProvider,
    builder: OnlineAttributesBuilder,
) -> OnlinePipeline {
    PipelineBuilder::new()
        .rollup_config(rollup_config)
        .dap_source(dap_source)
        .l2_chain_provider(l2_chain_provider)
        .chain_provider(chain_provider)
        .builder(builder)
        .build()
}
