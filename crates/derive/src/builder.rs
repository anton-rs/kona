//! Contains a concrete implementation of the [DerivationPipeline].

use crate::{
    stages::{
        AttributesBuilder, AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue,
        L1Retrieval, L1Traversal, NextAttributes,
    },
    traits::{ChainProvider, DataAvailabilityProvider, L2ChainProvider, OriginProvider},
    types::{L2AttributesWithParent, L2BlockInfo, RollupConfig, StageResult},
};
use alloc::sync::Arc;
use core::fmt::Debug;

/// The derivation pipeline is responsible for deriving L2 inputs from L1 data.
#[derive(Debug)]
pub struct DerivationPipeline<N: NextAttributes + Debug> {
    /// The attributes queue to retrieve the next attributes.
    pub attributes: N,
    /// A cursor for the [L2BlockInfo] parent to be used when pulling the next attributes.
    pub cursor: L2BlockInfo,
}

impl<N: NextAttributes + Debug + Send> DerivationPipeline<N> {
    /// Creates a new instance of the [DerivationPipeline].
    pub fn new(attributes: N, cursor: L2BlockInfo) -> Self {
        Self { attributes, cursor }
    }

    /// Set the [L2BlockInfo] cursor to be used when pulling the next attributes.
    pub fn set_cursor(&mut self, cursor: L2BlockInfo) {
        self.cursor = cursor;
    }

    /// Get the next attributes from the pipeline.
    pub async fn next(&mut self) -> StageResult<L2AttributesWithParent> {
        self.attributes.next_attributes(self.cursor).await
    }
}

impl<P, DAP, F, B> DerivationPipeline<KonaAttributes<P, DAP, F, B>>
where
    P: ChainProvider + Clone + Debug + Send,
    DAP: DataAvailabilityProvider + OriginProvider + Clone + Debug + Send,
    F: L2ChainProvider + Clone + Debug + Send,
    B: AttributesBuilder + Clone + Debug + Send,
{
    /// Creates a new instance of the [DerivationPipeline] from the given attributes.
    pub fn new_online_pipeline(
        attributes: KonaAttributes<P, DAP, F, B>,
        cursor: L2BlockInfo,
    ) -> Self {
        Self::new(attributes, cursor)
    }
}

/// [KonaDerivationPipeline] is a concrete [DerivationPipeline] type.
pub type KonaDerivationPipeline<P, DAP, F, B> = DerivationPipeline<KonaAttributes<P, DAP, F, B>>;

/// [KonaAttributes] is a concrete [NextAttributes] type.
pub type KonaAttributes<P, DAP, F, B> = AttributesQueue<
    BatchQueue<ChannelReader<ChannelBank<FrameQueue<L1Retrieval<DAP, L1Traversal<P>>>>>, F>,
    B,
>;

/// Creates a new [KonaAttributes] instance.
pub fn new_online_pipeline<P, DAP, F, B>(
    rollup_config: Arc<RollupConfig>,
    chain_provider: P,
    dap_source: DAP,
    fetcher: F,
    builder: B,
) -> KonaAttributes<P, DAP, F, B>
where
    P: ChainProvider + Clone + Debug + Send,
    DAP: DataAvailabilityProvider + OriginProvider + Clone + Debug + Send,
    F: L2ChainProvider + Clone + Debug + Send,
    B: AttributesBuilder + Clone + Debug + Send,
{
    let l1_traversal = L1Traversal::new(chain_provider, rollup_config.clone());
    let l1_retrieval = L1Retrieval::new(l1_traversal, dap_source);
    let frame_queue = FrameQueue::new(l1_retrieval);
    let channel_bank = ChannelBank::new(rollup_config.clone(), frame_queue);
    let channel_reader = ChannelReader::new(channel_bank, rollup_config.clone());
    let batch_queue = BatchQueue::new(rollup_config.clone(), channel_reader, fetcher);
    AttributesQueue::new(*rollup_config, batch_queue, builder)
}
