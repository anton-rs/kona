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
    traits::{ChainProvider, L2ChainProvider},
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
pub async fn new_online_pipeline(
    rollup_config: Arc<RollupConfig>,
    mut chain_provider: AlloyChainProvider,
    dap_source: OnlineDataProvider,
    mut l2_chain_provider: AlloyL2ChainProvider,
    builder: OnlineAttributesBuilder,
) -> OnlinePipeline {
    // Fetch the block for the rollup config genesis hash.
    let tip = chain_provider
        .block_info_by_number(rollup_config.genesis.l1.number)
        .await
        .expect("Failed to fetch genesis L1 block info for pipeline tip");

    // Fetch the block info for the cursor.
    let cursor = l2_chain_provider
        .l2_block_info_by_number(rollup_config.genesis.l2.number)
        .await
        .expect("Failed to fetch genesis L2 block info for pipeline cursor");
    PipelineBuilder::new()
        .rollup_config(rollup_config)
        .dap_source(dap_source)
        .l2_chain_provider(l2_chain_provider)
        .chain_provider(chain_provider)
        .builder(builder)
        .start_cursor(cursor)
        .tip(tip)
        .build()
}
