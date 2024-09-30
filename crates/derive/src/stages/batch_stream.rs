//! This module contains the `BatchStream` stage.

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use tracing::trace;

use crate::{
    batch::{Batch, SingleBatch, SpanBatch},
    errors::{PipelineError, PipelineResult},
    stages::BatchQueueProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};

/// Provides [Batch]es for the [BatchStream] stage.
#[async_trait]
pub trait BatchStreamProvider {
    /// Returns the next [Batch] in the [BatchStream] stage.
    async fn next_batch(&mut self) -> PipelineResult<Batch>;

    /// Drains the recent `Channel` if an invalid span batch is found post-holocene.
    fn flush(&mut self);
}

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
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// The previous stage in the derivation pipeline.
    prev: P,
    /// There can only be a single staged span batch.
    span: Option<SpanBatch>,
    /// A buffer of single batches derived from the [SpanBatch].
    buffer: Vec<SingleBatch>,
    /// A reference to the rollup config, used to check
    /// if the [BatchStream] stage should be activated.
    config: Arc<RollupConfig>,
}

impl<P> BatchStream<P>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// Create a new [BatchStream] stage.
    pub const fn new(prev: P, config: Arc<RollupConfig>) -> Self {
        Self { prev, span: None, buffer: Vec::new(), config }
    }

    /// Returns if the [BatchStream] stage is active based on the
    /// origin timestamp and holocene activation timestamp.
    pub fn is_active(&self) -> PipelineResult<bool> {
        let origin = self.prev.origin().ok_or(PipelineError::MissingOrigin.crit())?;
        Ok(self.config.is_holocene_active(origin.timestamp))
    }

    /// Gets a [SingleBatch] from the in-memory buffer.
    pub fn get_single_batch(&mut self) -> Option<SingleBatch> {
        trace!(target: "batch_span", "Attempting to get a SingleBatch from buffer len: {}", self.buffer.len());
        unimplemented!()
    }
}

#[async_trait]
impl<P> BatchQueueProvider for BatchStream<P>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    fn flush(&mut self) {
        if self.is_active().unwrap_or(false) {
            self.buffer.clear();
        }
    }

    async fn next_batch(&mut self, _: L2BlockInfo, _: &[BlockInfo]) -> PipelineResult<Batch> {
        // If the stage is not active, "pass" the next batch
        // through this stage to the BatchQueue stage.
        if !self.is_active()? {
            trace!(target: "batch_span", "BatchStream stage is inactive, pass-through.");
            return self.prev.next_batch().await;
        }

        // First, attempt to pull a SinguleBatch out of the buffer.
        if let Some(b) = self.get_single_batch() {
            return Ok(Batch::Single(b));
        }

        // Safety: bubble up any errors from the batch reader.
        let batch = self.prev.next_batch().await?;

        // If the next batch is a singular batch, it is immediately
        // forwarded to the `BatchQueue` stage.
        let Batch::Span(b) = batch else {
            return Ok(batch);
        };

        // Set the current span batch.
        self.span = Some(b);

        // Attempt to pull a SingleBatch out of the SpanBatch.
        self.get_single_batch()
            .map_or_else(|| Err(PipelineError::NotEnoughData.temp()), |b| Ok(Batch::Single(b)))
    }
}

#[async_trait]
impl<P> OriginAdvancer for BatchStream<P>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

impl<P> OriginProvider for BatchStream<P>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> ResettableStage for BatchStream<P>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug + Send,
{
    async fn reset(&mut self, base: BlockInfo, cfg: &SystemConfig) -> PipelineResult<()> {
        self.prev.reset(base, cfg).await?;
        self.span.take();
        crate::inc!(STAGE_RESETS, &["batch-span"]);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        batch::SingleBatch,
        stages::test_utils::{CollectingLayer, MockBatchStreamProvider, TraceStorage},
    };
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    #[tokio::test]
    async fn test_batch_stream_inactive() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        let data = vec![Ok(Batch::Single(SingleBatch::default()))];
        let config = Arc::new(RollupConfig { holocene_time: Some(100), ..RollupConfig::default() });
        let prev = MockBatchStreamProvider::new(data);
        let mut stream = BatchStream::new(prev, config.clone());

        // The stage should not be active.
        assert!(!stream.is_active().unwrap());

        // The next batch should be passed through to the [BatchQueue] stage.
        let batch = stream.next_batch(Default::default(), &[]).await.unwrap();
        assert_eq!(batch, Batch::Single(SingleBatch::default()));

        let logs = trace_store.get_by_level(tracing::Level::TRACE);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("BatchStream stage is inactive, pass-through."));
    }
}
