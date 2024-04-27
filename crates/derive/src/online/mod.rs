//! Contains "online" implementations for providers.

use crate::{
    sources::DataSourceFactory,
    stages::{
        AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue, L1Retrieval,
        L1Traversal, NextAttributes, StatefulAttributesBuilder,
    },
    traits::PreviousStage,
    types::RollupConfig,
};

use alloc::sync::Arc;
use alloy_provider::ReqwestProvider;
use core::fmt::Debug;

#[cfg(feature = "online")]
use kona_plasma::OnlinePlasmaInputFetcher;

#[cfg(feature = "online")]
use kona_providers::{AlloyChainProvider, AlloyL2ChainProvider};

/// Creates a new [OnlineStageStack].
#[cfg(feature = "online")]
pub fn new_online_stack(
    rollup_config: Arc<RollupConfig>,
    chain_provider: AlloyChainProvider<ReqwestProvider>,
    dap_source: DataSourceFactory<
        AlloyChainProvider<ReqwestProvider>,
        OnlineBlobProvider<
            ReqwestProvider,
            OnlineBeaconClient<ReqwestProvider>,
            SimpleSlotDerivation,
        >,
        OnlinePlasmaInputFetcher,
    >,
    fetcher: AlloyL2ChainProvider<ReqwestProvider>,
    builder: StatefulAttributesBuilder<
        AlloyChainProvider<ReqwestProvider>,
        AlloyL2ChainProvider<ReqwestProvider>,
    >,
) -> impl NextAttributes + PreviousStage + Debug + Send {
    let l1_traversal = L1Traversal::new(chain_provider, rollup_config.clone());
    let l1_retrieval = L1Retrieval::new(l1_traversal, dap_source);
    let frame_queue = FrameQueue::new(l1_retrieval);
    let channel_bank = ChannelBank::new(rollup_config.clone(), frame_queue);
    let channel_reader = ChannelReader::new(channel_bank, rollup_config.clone());
    let batch_queue = BatchQueue::new(rollup_config.clone(), channel_reader, fetcher);
    AttributesQueue::new(*rollup_config, batch_queue, builder)
}

#[cfg(test)]
#[allow(unreachable_pub)]
pub mod test_utils;

mod beacon_client;
pub use beacon_client::{BeaconClient, OnlineBeaconClient};

mod blob_provider;
pub use blob_provider::{OnlineBlobProvider, SimpleSlotDerivation};

mod utils;
pub(crate) use utils::blobs_from_sidecars;
