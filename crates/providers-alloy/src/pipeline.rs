//! Helper to construct a [DerivationPipeline] using online types.

use kona_derive::{
    attributes::StatefulAttributesBuilder,
    pipeline::{DerivationPipeline, PipelineBuilder},
    sources::EthereumDataSource,
    stages::{
        AttributesQueue, BatchQueue, BatchStream, ChannelBank, ChannelReader, FrameQueue,
        L1Retrieval, L1Traversal,
    },
};
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::BlockInfo;
use std::sync::Arc;

use crate::{
    AlloyChainProvider, AlloyL2ChainProvider, OnlineBeaconClient, OnlineBlobProviderWithFallback,
    SimpleSlotDerivation,
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
        BatchStream<
            ChannelReader<
                ChannelBank<FrameQueue<L1Retrieval<DAP, L1Traversal<AlloyChainProvider>>>>,
            >,
            AlloyL2ChainProvider,
        >,
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
