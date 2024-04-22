//! Contains a stack of Stages for the [crate::DerivationPipeline].

use crate::{
    stages::{
        AttributesBuilder, AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue,
        L1Retrieval, L1Traversal, NextAttributes,
    },
    traits::{
        ChainProvider, DataAvailabilityProvider, L2ChainProvider, OriginProvider, ResettableStage,
    },
    types::{
        BlockInfo, L2AttributesWithParent, L2BlockInfo, RollupConfig, StageError, StageResult,
        SystemConfig,
    },
};
use alloc::{boxed::Box, sync::Arc};
use async_trait::async_trait;
use core::fmt::Debug;
use spin::Mutex;

/// The [AttributesQueue] type alias.
pub type AttributesQueueType<P, DAP, F, B> = AttributesQueue<
    BatchQueue<ChannelReader<ChannelBank<FrameQueue<L1Retrieval<DAP, L1Traversal<P>>>>>, F>,
    B,
>;

/// An online stack of stages.
#[derive(Debug)]
pub struct OnlineStageStack<P, DAP, F, B>
where
    P: ChainProvider + Clone + Debug + Send,
    DAP: DataAvailabilityProvider + OriginProvider + Clone + Debug + Send,
    F: L2ChainProvider + Clone + Debug + Send,
    B: AttributesBuilder + Clone + Debug + Send,
{
    /// Flag to tell the L1Traversal stage to advance to the next L1 block.
    pub advance: Arc<Mutex<bool>>,
    /// The [AttributesQueue] stage.
    pub attributes: AttributesQueueType<P, DAP, F, B>,
}

impl<P, DAP, F, B> OnlineStageStack<P, DAP, F, B>
where
    P: ChainProvider + Clone + Debug + Send,
    DAP: DataAvailabilityProvider + OriginProvider + Clone + Debug + Send,
    F: L2ChainProvider + Clone + Debug + Send,
    B: AttributesBuilder + Clone + Debug + Send,
{
    /// Creates a new [OnlineStageStack].
    pub fn new(
        rollup_config: Arc<RollupConfig>,
        chain_provider: P,
        dap_source: DAP,
        fetcher: F,
        builder: B,
    ) -> Self {
        let advance = Arc::new(Mutex::new(false));
        let l1_traversal =
            L1Traversal::new(chain_provider, Arc::clone(&advance), rollup_config.clone());
        let l1_retrieval = L1Retrieval::new(l1_traversal, dap_source);
        let frame_queue = FrameQueue::new(l1_retrieval);
        let channel_bank = ChannelBank::new(rollup_config.clone(), frame_queue);
        let channel_reader = ChannelReader::new(channel_bank, rollup_config.clone());
        let batch_queue = BatchQueue::new(rollup_config.clone(), channel_reader, fetcher);
        let attributes = AttributesQueue::new(*rollup_config, batch_queue, builder);
        Self { advance, attributes }
    }
}

#[async_trait]
impl<P, DAP, F, B> NextAttributes for OnlineStageStack<P, DAP, F, B>
where
    P: ChainProvider + Clone + Debug + Send,
    DAP: DataAvailabilityProvider + OriginProvider + Clone + Debug + Send,
    F: L2ChainProvider + Clone + Debug + Send,
    B: AttributesBuilder + Clone + Debug + Send,
{
    /// Advances the pipeline to the next attributes.
    async fn next_attributes(
        &mut self,
        parent: L2BlockInfo,
    ) -> StageResult<L2AttributesWithParent> {
        match self.attributes.next_attributes(parent).await {
            Ok(a) => {
                tracing::info!("attributes queue stage step returned l2 attributes");
                tracing::info!("prepared L2 attributes: {:?}", a);
                return Ok(a);
            }
            Err(StageError::Eof) => {
                tracing::info!("attributes queue stage complete");
                let mut advance = self.advance.lock();
                *advance = true;
                return Err(StageError::Eof);
            }
            // TODO: match on the EngineELSyncing error here and log
            Err(err) => {
                tracing::error!("attributes queue stage failed: {:?}", err);
                return Err(err);
            }
        }
    }
}

#[async_trait]
impl<P, DAP, F, B> ResettableStage for OnlineStageStack<P, DAP, F, B>
where
    P: ChainProvider + Clone + Debug + Send,
    DAP: DataAvailabilityProvider + OriginProvider + Clone + Debug + Send,
    F: L2ChainProvider + Clone + Debug + Send,
    B: AttributesBuilder + Clone + Debug + Send,
{
    /// Resets all stages in the stack.
    async fn reset(&mut self, bi: BlockInfo, sc: &SystemConfig) -> StageResult<()> {
        match self.attributes.reset(bi, sc).await {
            Ok(()) => {
                tracing::info!("Stages reset");
            }
            Err(StageError::Eof) => {
                tracing::info!("Stages reset with EOF");
            }
            Err(err) => {
                tracing::error!("Stages reset failed: {:?}", err);
                return Err(err);
            }
        }
        Ok(())
    }
}
