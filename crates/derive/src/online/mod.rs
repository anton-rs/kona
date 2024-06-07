//! Contains "online" implementations for providers.

use crate::{
    stages::{
        AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue, L1Retrieval,
        L1Traversal, StatefulAttributesBuilder,
    },
    traits::DataAvailabilityProvider,
    types::RollupConfig,
};
use alloc::sync::Arc;
use core::fmt::Debug;

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

/// Creates a new online stack.
pub fn new_online_stack<DAP>(
    rollup_config: Arc<RollupConfig>,
    chain_provider: AlloyChainProvider,
    dap_source: DAP,
    fetcher: AlloyL2ChainProvider,
    builder: OnlineAttributesBuilder,
) -> OnlineAttributesQueue<DAP>
where
    DAP: DataAvailabilityProvider + Debug + Send,
{
    let l1_traversal = L1Traversal::new(chain_provider, rollup_config.clone());
    let l1_retrieval = L1Retrieval::new(l1_traversal, dap_source);
    let frame_queue = FrameQueue::new(l1_retrieval);
    let channel_bank = ChannelBank::new(rollup_config.clone(), frame_queue);
    let channel_reader = ChannelReader::new(channel_bank, rollup_config.clone());
    let batch_queue = BatchQueue::new(rollup_config.clone(), channel_reader, fetcher);
    AttributesQueue::new(*rollup_config, batch_queue, builder)
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

mod beacon_client;
pub use beacon_client::{BeaconClient, OnlineBeaconClient};

mod alloy_providers;
pub use alloy_providers::{AlloyChainProvider, AlloyL2ChainProvider, ReqwestClient};

mod blob_provider;
pub use blob_provider::{OnlineBlobProvider, SimpleSlotDerivation};
