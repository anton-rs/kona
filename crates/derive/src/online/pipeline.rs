//! Contains online pipeline types.

use super::{
    AlloyChainProvider, AlloyL2ChainProvider, BlockInfo, DerivationPipeline, EthereumDataSource,
    OnlineBeaconClient, OnlineBlobProviderWithFallback, PipelineBuilder, RollupConfig,
    SimpleSlotDerivation, StatefulAttributesBuilder,
};
use alloc::sync::Arc;
// Pipeline internal stages aren't re-exported at the module-level.
use crate::stages::{
    AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue, L1Retrieval, L1Traversal,
};

/// An online derivation pipeline.
pub type OnlinePipeline =
    DerivationPipeline<OnlineAttributesQueue<OnlineDataProvider>, AlloyL2ChainProvider>;

/// An `online` Ethereum data source.
pub type OnlineDataProvider = EthereumDataSource<
    AlloyChainProvider,
    OnlineBlobProviderWithFallback<OnlineBeaconClient, OnlineBeaconClient, SimpleSlotDerivation>,
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
    origin: BlockInfo,
) -> OnlinePipeline {
    PipelineBuilder::new()
        .rollup_config(rollup_config)
        .dap_source(dap_source)
        .l2_chain_provider(l2_chain_provider)
        .chain_provider(chain_provider)
        .builder(builder)
        .origin(origin)
        .build()
}
