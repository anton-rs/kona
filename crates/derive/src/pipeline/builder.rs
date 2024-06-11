//! Contains the `PipelineBuilder` object that is used to build a `DerivationPipeline`.

use super::{
    AttributesBuilder, ChainProvider, DataAvailabilityProvider, DerivationPipeline, L2ChainProvider,
};
use crate::stages::{
    AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue, L1Retrieval, L1Traversal,
};
use alloc::{collections::VecDeque, sync::Arc};
use core::fmt::Debug;
use kona_primitives::{BlockInfo, L2BlockInfo, RollupConfig};

type L1TraversalStage<P> = L1Traversal<P>;
type L1RetrievalStage<DAP, P> = L1Retrieval<DAP, L1TraversalStage<P>>;
type FrameQueueStage<DAP, P> = FrameQueue<L1RetrievalStage<DAP, P>>;
type ChannelBankStage<DAP, P> = ChannelBank<FrameQueueStage<DAP, P>>;
type ChannelReaderStage<DAP, P> = ChannelReader<ChannelBankStage<DAP, P>>;
type BatchQueueStage<DAP, P, T> = BatchQueue<ChannelReaderStage<DAP, P>, T>;
type AttributesQueueStage<DAP, P, T, B> = AttributesQueue<BatchQueueStage<DAP, P, T>, B>;

/// The `PipelineBuilder` constructs a [DerivationPipeline] using a builder pattern.
#[cfg_attr(feature = "online", doc = include_str!("../../USAGE.md"))]
#[derive(Debug)]
pub struct PipelineBuilder<B, P, T, D>
where
    B: AttributesBuilder + Send + Debug,
    P: ChainProvider + Send + Sync + Debug,
    T: L2ChainProvider + Clone + Send + Sync + Debug,
    D: DataAvailabilityProvider + Send + Sync + Debug,
{
    l2_chain_provider: Option<T>,
    dap_source: Option<D>,
    chain_provider: Option<P>,
    builder: Option<B>,
    rollup_config: Option<Arc<RollupConfig>>,
    start_cursor: Option<L2BlockInfo>,
    tip: Option<BlockInfo>,
}

impl<B, P, T, D> Default for PipelineBuilder<B, P, T, D>
where
    B: AttributesBuilder + Send + Debug,
    P: ChainProvider + Send + Sync + Debug,
    T: L2ChainProvider + Clone + Send + Sync + Debug,
    D: DataAvailabilityProvider + Send + Sync + Debug,
{
    fn default() -> Self {
        Self {
            l2_chain_provider: None,
            dap_source: None,
            chain_provider: None,
            builder: None,
            tip: None,
            rollup_config: None,
            start_cursor: None,
        }
    }
}

impl<B, P, T, D> PipelineBuilder<B, P, T, D>
where
    B: AttributesBuilder + Send + Debug,
    P: ChainProvider + Send + Sync + Debug,
    T: L2ChainProvider + Clone + Send + Sync + Debug,
    D: DataAvailabilityProvider + Send + Sync + Debug,
{
    /// Creates a new pipeline builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the rollup config for the pipeline.
    pub fn rollup_config(mut self, rollup_config: Arc<RollupConfig>) -> Self {
        self.rollup_config = Some(rollup_config);
        self
    }

    /// Sets the tip for the pipeline.
    pub fn tip(mut self, tip: BlockInfo) -> Self {
        self.tip = Some(tip);
        self
    }

    /// Sets the start cursor for the pipeline.
    pub fn start_cursor(mut self, cursor: L2BlockInfo) -> Self {
        self.start_cursor = Some(cursor);
        self
    }

    /// Sets the data availability provider for the pipeline.
    pub fn dap_source(mut self, dap_source: D) -> Self {
        self.dap_source = Some(dap_source);
        self
    }

    /// Sets the builder for the pipeline.
    pub fn builder(mut self, builder: B) -> Self {
        self.builder = Some(builder);
        self
    }

    /// Sets the l2 chain provider for the pipeline.
    pub fn l2_chain_provider(mut self, l2_chain_provider: T) -> Self {
        self.l2_chain_provider = Some(l2_chain_provider);
        self
    }

    /// Sets the chain provider for the pipeline.
    pub fn chain_provider(mut self, chain_provider: P) -> Self {
        self.chain_provider = Some(chain_provider);
        self
    }

    /// Builds the pipeline.
    pub fn build(self) -> DerivationPipeline<AttributesQueueStage<D, P, T, B>, T> {
        self.into()
    }
}

impl<B, P, T, D> From<PipelineBuilder<B, P, T, D>>
    for DerivationPipeline<AttributesQueueStage<D, P, T, B>, T>
where
    B: AttributesBuilder + Send + Debug,
    P: ChainProvider + Send + Sync + Debug,
    T: L2ChainProvider + Clone + Send + Sync + Debug,
    D: DataAvailabilityProvider + Send + Sync + Debug,
{
    fn from(builder: PipelineBuilder<B, P, T, D>) -> Self {
        // Extract the builder fields.
        let rollup_config = builder.rollup_config.expect("rollup_config must be set");
        let chain_provider = builder.chain_provider.expect("chain_provider must be set");
        let l2_chain_provider = builder.l2_chain_provider.expect("chain_provider must be set");
        let dap_source = builder.dap_source.expect("dap_source must be set");
        let attributes_builder = builder.builder.expect("builder must be set");

        // Compose the stage stack.
        let mut l1_traversal = L1Traversal::new(chain_provider, Arc::clone(&rollup_config));
        l1_traversal.block = builder.tip;
        let l1_retrieval = L1Retrieval::new(l1_traversal, dap_source);
        let frame_queue = FrameQueue::new(l1_retrieval);
        let channel_bank = ChannelBank::new(Arc::clone(&rollup_config), frame_queue);
        let channel_reader = ChannelReader::new(channel_bank, Arc::clone(&rollup_config));
        let batch_queue =
            BatchQueue::new(rollup_config.clone(), channel_reader, l2_chain_provider.clone());
        let attributes = AttributesQueue::new(*rollup_config, batch_queue, attributes_builder);

        // Create the pipeline.
        DerivationPipeline {
            attributes,
            tip: builder.tip.unwrap_or_default(),
            prepared: VecDeque::new(),
            cursor: builder.start_cursor.unwrap_or_default(),
            rollup_config,
            l2_chain_provider,
        }
    }
}
