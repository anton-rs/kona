//! This module contains the `BatchStream` stage.

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use tracing::{info, trace};

use crate::{
    batch::{Batch, SingleBatch, SpanBatch},
    errors::{PipelineEncodingError, PipelineError, PipelineResult},
    stages::BatchQueueProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};

/// [BatchStream] stage in the derivation pipeline.
///
/// This stage is introduced in the [Holocene] hardfork.
/// It slots in between the [ChannelReader] and [BatchQueue]
/// stages, buffering span batches until they are validated.
///
/// [Holocene]: https://specs.optimism.io/protocol/holocene/overview.html
/// [ChannelReader]: crate::stages::ChannelReader
/// [BatchQueue]: crate::stages::BatchQueue
#[derive(Debug)]
pub struct BatchStream<P>
where
    P: BatchQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// The previous stage in the derivation pipeline.
    prev: P,
    /// A reference to the rollup config, used to check
    /// if the [BatchStream] stage should be activated.
    config: Arc<RollupConfig>,
    /// The l1 block ref
    origin: Option<BlockInfo>,
    /// There can only be a single staged span batch.
    span: Option<SpanBatch>,
    /// A buffer of single batches derived from the [SpanBatch].
    buffer: Vec<SingleBatch>,
    /// A consecutive, time-centric window of L1 Blocks.
    /// Every L1 origin of unsafe L2 Blocks must be included in this list.
    /// If every L2 Block corresponding to a single L1 Block becomes safe,
    /// the block is popped from this list.
    /// If new L2 Block's L1 origin is not included in this list, fetch and
    /// push it to the list.
    l1_blocks: Vec<BlockInfo>,
}

impl<P> BatchStream<P>
where
    P: BatchQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// Create a new [BatchStream] stage.
    pub const fn new(prev: P, config: Arc<RollupConfig>) -> Self {
        Self { prev, config, origin: None, span: None, buffer: Vec::new(), l1_blocks: Vec::new() }
    }

    /// Returns if the [BatchStream] stage is active based on the
    /// origin timestamp and holocene activation timestamp.
    pub fn is_active(&self) -> PipelineResult<bool> {
        let origin = self.prev.origin().ok_or(PipelineError::MissingOrigin.crit())?;
        Ok(self.config.is_holocene_active(origin.timestamp))
    }

    /// Updates the in-memory list of L1 Blocks.
    pub fn update_l1_blocks(&mut self, parent: L2BlockInfo) -> PipelineResult<()> {
        // If the epoch is advanced, update the l1 blocks.
        // Advancing epoch must be done after the pipeline successfully applies the entire span
        // batch to the chain.
        // Because the span batch can be reverted during processing the batch, then we must
        // preserve existing l1 blocks to verify the epochs of the next candidate batch.
        if !self.l1_blocks.is_empty() && parent.l1_origin.number > self.l1_blocks[0].number {
            for (i, block) in self.l1_blocks.iter().enumerate() {
                if parent.l1_origin.number == block.number {
                    self.l1_blocks.drain(0..i);
                    info!(target: "batch-stream", "Advancing epoch");
                    break;
                }
            }
            // If the origin of the parent block is not included, we must advance the origin.
        }

        // NOTE: The origin is used to determine if it's behind.
        // It is the future origin that gets saved into the l1 blocks array.
        // We always update the origin of this stage if it's not the same so
        // after the update code runs, this is consistent.
        let origin_behind =
            self.prev.origin().map_or(true, |origin| origin.number < parent.l1_origin.number);

        // Advance the origin if needed.
        // The entire pipeline has the same origin.
        // Batches prior to the l1 origin of the l2 safe head are not accepted.
        if self.origin != self.prev.origin() {
            self.origin = self.prev.origin();
            if !origin_behind {
                let origin = match self.origin.as_ref().ok_or(PipelineError::MissingOrigin.crit()) {
                    Ok(o) => o,
                    Err(e) => return Err(e),
                };
                self.l1_blocks.push(*origin);
            } else {
                // This is to handle the special case of startup.
                // At startup, the batch queue is reset and includes the
                // l1 origin. That is the only time where immediately after
                // reset is called, the origin behind is false.
                self.l1_blocks.clear();
            }
            info!(target: "batch-stream", "Advancing batch queue origin: {:?}", self.origin);
        }

        Ok(())
    }

    /// Gets a [SingleBatch] from the in-memory buffer.
    pub fn get_single_batch(&mut self, parent: L2BlockInfo) -> Option<SingleBatch> {
        if self.buffer.is_empty() {
            return None;
        }
        let mut next = self.buffer.remove(0);
        next.parent_hash = parent.block_info.hash;
        Some(next)
    }
}

#[async_trait]
impl<P> BatchQueueProvider for BatchStream<P>
where
    P: BatchQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    fn flush(&mut self) {
        if self.is_active().unwrap_or(false) {
            self.buffer.clear();
        }
    }

    async fn next_batch(&mut self, parent: L2BlockInfo) -> PipelineResult<Batch> {
        // If the stage is not active, "pass" the next batch
        // through this stage to the BatchQueue stage.
        if !self.is_active()? {
            trace!(target: "batch_span", "BatchStream stage is inactive, pass-through.");
            return self.prev.next_batch(parent).await;
        }

        // First update the in-memory list of L1 Block origins.
        self.update_l1_blocks(parent)?;

        // First, attempt to pull a SinguleBatch out of the buffer.
        if let Some(b) = self.get_single_batch(parent) {
            return Ok(Batch::Single(b));
        }

        // Safety: bubble up any errors from the batch reader.
        let batch = self.prev.next_batch(parent).await?;

        // If the next batch is a singular batch, it is immediately
        // forwarded to the `BatchQueue` stage.
        let Batch::Span(b) = batch else {
            return Ok(batch);
        };

        // Validate the span batch.
        // If it is invalid, drop the batch and flush the channel.
        //
        // See: <https://specs.optimism.io/protocol/holocene/derivation.html#span-batches>
        if !b.is_batch_holocene_valid(parent, self.config.block_time) {
            self.prev.flush();
            return Err(PipelineError::InvalidSpanBatch.temp());
        }

        // Extract the singular batches from the span batch.
        let batches = match b.get_singular_batches(&self.l1_blocks, parent).map_err(|e| {
            PipelineError::BadEncoding(PipelineEncodingError::SpanBatchError(e)).crit()
        }) {
            Ok(b) => b,
            Err(e) => {
                return Err(e);
            }
        };
        self.buffer = batches;

        // Attempt to pull a SingleBatch out of the SpanBatch.
        self.get_single_batch(parent)
            .map_or_else(|| Err(PipelineError::NotEnoughData.temp()), |b| Ok(Batch::Single(b)))
    }
}

#[async_trait]
impl<P> OriginAdvancer for BatchStream<P>
where
    P: BatchQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

impl<P> OriginProvider for BatchStream<P>
where
    P: BatchQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> ResettableStage for BatchStream<P>
where
    P: BatchQueueProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug + Send,
{
    async fn reset(&mut self, base: BlockInfo, cfg: &SystemConfig) -> PipelineResult<()> {
        self.prev.reset(base, cfg).await?;
        self.origin = Some(base);
        self.span.take();
        // Include the new origin as an origin to build on.
        // This is only for the initialization case.
        // During normal resets we will later throw out this block.
        self.l1_blocks.clear();
        self.l1_blocks.push(base);
        crate::inc!(STAGE_RESETS, &["batch-span"]);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        batch::SingleBatch,
        stages::test_utils::{CollectingLayer, MockBatchQueueProvider, TraceStorage},
    };
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    #[tokio::test]
    async fn test_batch_stream_inactive() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        let data = vec![Ok(Batch::Single(SingleBatch::default()))];
        let config = Arc::new(RollupConfig { holocene_time: Some(100), ..RollupConfig::default() });
        let prev = MockBatchQueueProvider::new(data);
        let mut stream = BatchStream::new(prev, config.clone());

        // The stage should not be active.
        assert!(!stream.is_active().unwrap());

        // The next batch should be passed through to the [BatchQueue] stage.
        let batch = stream.next_batch(Default::default()).await.unwrap();
        assert_eq!(batch, Batch::Single(SingleBatch::default()));

        let logs = trace_store.get_by_level(tracing::Level::TRACE);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("BatchStream stage is inactive, pass-through."));
    }
}
