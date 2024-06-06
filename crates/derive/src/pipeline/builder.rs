//! Contains the `PipelineBuilder` object that is used to build a `DerivationPipeline`.

use super::{DerivationPipeline, NextAttributes, AttributesBuilder, OriginAdvancer, ResetProvider, ResettableStage};
use alloc::collections::VecDeque;
use core::fmt::Debug;
use kona_primitives::L2BlockInfo;

/// The PipelineBuilder constructs a [DerivationPipeline].
#[derive(Debug)]
pub struct PipelineBuilder<S, R, B>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
    B: AttributesBuilder + Debug,
{
    rollup_config: Option<Arc<RollupConfig>>,
    reset: Option<R>,
    start_cursor: Option<L2BlockInfo>,
}

impl<S, R> PipelineBuilder<S, R>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
{
    /// Sets the attributes for the pipeline.
    pub fn attributes(mut self, attributes: S) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Sets the reset provider for the pipeline.
    pub fn reset(mut self, reset: R) -> Self {
        self.reset = Some(reset);
        self
    }

    /// Sets the start cursor for the pipeline.
    pub fn start_cursor(mut self, cursor: L2BlockInfo) -> Self {
        self.start_cursor = Some(cursor);
        self
    }

    /// Builds the pipeline.
    pub fn build(self) -> DerivationPipeline<S, R> {
        self.into()
    }
}

impl<S, R> From<PipelineBuilder<S, R>> for DerivationPipeline<S, R>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
{
    fn from(builder: PipelineBuilder<S, R>) -> Self {
        // Instantiates and link all the stages.
        let chain_provider = ExExChainProvider::new(Arc::clone(&self.chain_provider));
        let l1_traversal = L1Traversal::new(chain_provider, Arc::clone(&rollup_config));
        let l1_retrieval = L1Retrieval::new(l1_traversal, dap_source);
        let frame_queue = FrameQueue::new(l1_retrieval);
        let channel_bank = ChannelBank::new(Arc::clone(&rollup_config), frame_queue);
        let channel_reader = ChannelReader::new(channel_bank, Arc::clone(&rollup_config));
        let batch_queue = BatchQueue::new(rollup_config.clone(), channel_reader, l2_chain_provider);
        let queue = AttributesQueue::new(*rollup_config, batch_queue, builder);

        let attributes = builder.attributes.expect("attributes must be set");
        let reset = builder.reset.expect("reset must be set");
        let start_cursor = builder.start_cursor.expect("start_cursor must be set");

        DerivationPipeline {
            attributes,
            reset,
            prepared: VecDeque::new(),
            needs_reset: false,
            cursor: start_cursor,
        }
    }
}
