//! Helper to construct a [DerivationPipeline] using online types.

use kona_derive::{
    attributes::StatefulAttributesBuilder,
    pipeline::{DerivationPipeline, PipelineBuilder},
    sources::EthereumDataSource,
    stages::{
        AttributesQueue, BatchProvider, BatchStream, ChannelProvider, ChannelReader, FrameQueue,
        L1Retrieval, L1Traversal,
    },
};
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::BlockInfo;
use std::sync::Arc;

use crate::{
    AlloyChainProvider, AlloyL2ChainProvider, OnlineBeaconClient, OnlineBlobProviderWithFallback,
};

/// An online derivation pipeline.
pub type OnlinePipeline =
    DerivationPipeline<OnlineAttributesQueue<OnlineDataProvider>, AlloyL2ChainProvider>;

/// An `online` Ethereum data source.
pub type OnlineDataProvider = EthereumDataSource<
    AlloyChainProvider,
    OnlineBlobProviderWithFallback<OnlineBeaconClient, OnlineBeaconClient>,
>;

/// An `online` payload attributes builder for the `AttributesQueue` stage of the derivation
/// pipeline.
pub type OnlineAttributesBuilder =
    StatefulAttributesBuilder<AlloyChainProvider, AlloyL2ChainProvider>;

/// An `online` attributes queue for the derivation pipeline.
pub type OnlineAttributesQueue<DAP> = AttributesQueue<
    BatchProvider<
        BatchStream<
            ChannelReader<
                ChannelProvider<FrameQueue<L1Retrieval<DAP, L1Traversal<AlloyChainProvider>>>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OnlineBlobProvider;
    use kona_derive::prelude::OriginProvider;

    #[test]
    fn test_new_online_pipeline() {
        let rollup_config = Arc::new(RollupConfig::default());
        let chain_provider =
            AlloyChainProvider::new_http("http://127.0.0.1:8545".try_into().unwrap());
        let l2_chain_provider = AlloyL2ChainProvider::new_http(
            "http://127.0.0.1:9545".try_into().unwrap(),
            rollup_config.clone(),
        );
        let beacon_client = OnlineBeaconClient::new_http("http://127.0.0.1:5555".into());
        let blob_provider = OnlineBlobProvider::new(beacon_client, None, None);
        let blob_provider = OnlineBlobProviderWithFallback::new(blob_provider, None);
        let dap_source = EthereumDataSource::new_from_parts(
            chain_provider.clone(),
            blob_provider,
            &rollup_config,
        );
        let builder = StatefulAttributesBuilder::new(
            rollup_config.clone(),
            l2_chain_provider.clone(),
            chain_provider.clone(),
        );
        let origin = BlockInfo::default();

        let pipeline = new_online_pipeline(
            rollup_config.clone(),
            chain_provider,
            dap_source,
            l2_chain_provider,
            builder,
            origin,
        );

        assert_eq!(pipeline.rollup_config, rollup_config);
        assert_eq!(pipeline.origin(), Some(origin));
    }
}
