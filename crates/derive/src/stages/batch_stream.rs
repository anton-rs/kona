//! This module contains the `BatchStream` stage.

use crate::{
    batch::{Batch, BatchValidity, SingleBatch, SpanBatch},
    errors::{PipelineEncodingError, PipelineError, PipelineResult},
    pipeline::L2ChainProvider,
    stages::BatchQueueProvider,
    traits::{OriginAdvancer, OriginProvider, PreviousStage, ResettableStage},
};
use alloc::{boxed::Box, collections::VecDeque, sync::Arc};
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use tracing::trace;

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
pub struct BatchStream<P, BF>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
    BF: L2ChainProvider + Debug,
{
    /// The previous stage in the derivation pipeline.
    prev: P,
    /// There can only be a single staged span batch.
    span: Option<SpanBatch>,
    /// A buffer of single batches derived from the [SpanBatch].
    buffer: VecDeque<SingleBatch>,
    /// A reference to the rollup config, used to check
    /// if the [BatchStream] stage should be activated.
    config: Arc<RollupConfig>,
    /// Used to validate the batches.
    fetcher: BF,
}

impl<P, BF> BatchStream<P, BF>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
    BF: L2ChainProvider + Debug,
{
    /// Create a new [BatchStream] stage.
    pub const fn new(prev: P, config: Arc<RollupConfig>, fetcher: BF) -> Self {
        Self { prev, span: None, buffer: VecDeque::new(), config, fetcher }
    }

    /// Returns if the [BatchStream] stage is active based on the
    /// origin timestamp and holocene activation timestamp.
    pub fn is_active(&self) -> PipelineResult<bool> {
        let origin = self.prev.origin().ok_or(PipelineError::MissingOrigin.crit())?;
        Ok(self.config.is_holocene_active(origin.timestamp))
    }

    /// Gets a [SingleBatch] from the in-memory buffer.
    pub fn get_single_batch(
        &mut self,
        parent: L2BlockInfo,
        l1_origins: &[BlockInfo],
    ) -> PipelineResult<SingleBatch> {
        trace!(target: "batch_span", "Attempting to get a SingleBatch from buffer len: {}", self.buffer.len());

        self.try_hydrate_buffer(parent, l1_origins)?;
        self.buffer.pop_front().ok_or_else(|| PipelineError::NotEnoughData.temp())
    }

    /// Hydrates the buffer with single batches derived from the span batch, if there is one
    /// queued up.
    pub fn try_hydrate_buffer(
        &mut self,
        parent: L2BlockInfo,
        l1_origins: &[BlockInfo],
    ) -> PipelineResult<()> {
        if let Some(span) = self.span.take() {
            self.buffer.extend(
                span.get_singular_batches(l1_origins, parent).map_err(|e| {
                    PipelineError::BadEncoding(PipelineEncodingError::from(e)).crit()
                })?,
            );
        }
        Ok(())
    }
}

#[async_trait]
impl<P, BF> BatchQueueProvider for BatchStream<P, BF>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
    BF: L2ChainProvider + Send + Debug,
{
    fn flush(&mut self) {
        if self.is_active().unwrap_or(false) {
            self.prev.flush();
            self.span = None;
            self.buffer.clear();
        }
    }

    async fn next_batch(
        &mut self,
        parent: L2BlockInfo,
        l1_origins: &[BlockInfo],
    ) -> PipelineResult<Batch> {
        // If the stage is not active, "pass" the next batch
        // through this stage to the BatchQueue stage.
        if !self.is_active()? {
            trace!(target: "batch_span", "BatchStream stage is inactive, pass-through.");
            return self.prev.next_batch().await;
        }

        // If the buffer is empty, attempt to pull a batch from the previous stage.
        if self.buffer.is_empty() {
            // Safety: bubble up any errors from the batch reader.
            let batch = self.prev.next_batch().await?;

            // If the next batch is a singular batch, it is immediately
            // forwarded to the `BatchQueue` stage. Otherwise, we buffer
            // the span batch in this stage if it passes the validity checks.
            match batch {
                Batch::Single(b) => return Ok(Batch::Single(b)),
                Batch::Span(b) => {
                    let validity = b
                        .check_batch_prefix(
                            self.config.as_ref(),
                            l1_origins,
                            parent.block_info,
                            &mut self.fetcher,
                        )
                        .await;

                    match validity {
                        BatchValidity::Accept => self.span = Some(b),
                        BatchValidity::Drop => {
                            // Flush the stage.
                            self.flush();

                            return Err(PipelineError::Eof.temp());
                        }
                        BatchValidity::Undecided | BatchValidity::Future => {
                            return Err(PipelineError::NotEnoughData.temp())
                        }
                    }
                }
            }
        }

        // Attempt to pull a SingleBatch out of the SpanBatch.
        self.get_single_batch(parent, l1_origins).map(Batch::Single)
    }
}

impl<P, B> PreviousStage for BatchStream<P, B>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
    B: L2ChainProvider + Send + Debug,
{
    type Previous = P;

    fn prev(&self) -> Option<&Self::Previous> {
        Some(&self.prev)
    }

    fn prev_mut(&mut self) -> Option<&mut Self::Previous> {
        Some(&mut self.prev)
    }
}

#[async_trait]
impl<P, BF> OriginAdvancer for BatchStream<P, BF>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
    BF: L2ChainProvider + Send + Debug,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

impl<P, BF> OriginProvider for BatchStream<P, BF>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
    BF: L2ChainProvider + Debug + Send,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P, BF> ResettableStage for BatchStream<P, BF>
where
    P: BatchStreamProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug + Send,
    BF: L2ChainProvider + Send + Debug,
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
        batch::{SingleBatch, SpanBatchElement},
        stages::test_utils::{CollectingLayer, MockBatchStreamProvider, TraceStorage},
        traits::test_utils::TestL2ChainProvider,
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
        let mut stream = BatchStream::new(prev, config.clone(), TestL2ChainProvider::default());

        // The stage should not be active.
        assert!(!stream.is_active().unwrap());

        // The next batch should be passed through to the [BatchQueue] stage.
        let batch = stream.next_batch(Default::default(), &[]).await.unwrap();
        assert_eq!(batch, Batch::Single(SingleBatch::default()));

        let logs = trace_store.get_by_level(tracing::Level::TRACE);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("BatchStream stage is inactive, pass-through."));
    }

    #[tokio::test]
    async fn test_span_buffer() {
        let mock_batch = SpanBatch {
            batches: vec![
                SpanBatchElement { epoch_num: 10, timestamp: 2, ..Default::default() },
                SpanBatchElement { epoch_num: 10, timestamp: 4, ..Default::default() },
            ],
            ..Default::default()
        };
        let mock_origins = [BlockInfo { number: 10, timestamp: 12, ..Default::default() }];

        let data = vec![Ok(Batch::Span(mock_batch.clone()))];
        let config = Arc::new(RollupConfig {
            holocene_time: Some(0),
            block_time: 2,
            ..RollupConfig::default()
        });
        let prev = MockBatchStreamProvider::new(data);
        let provider = TestL2ChainProvider::default();
        let mut stream = BatchStream::new(prev, config.clone(), provider);

        // The stage should be active.
        assert!(stream.is_active().unwrap());

        // The next batches should be single batches derived from the span batch.
        let batch = stream.next_batch(Default::default(), &mock_origins).await.unwrap();
        if let Batch::Single(single) = batch {
            assert_eq!(single.epoch_num, 10);
            assert_eq!(single.timestamp, 2);
        } else {
            panic!("Wrong batch type");
        }

        let batch = stream.next_batch(Default::default(), &mock_origins).await.unwrap();
        if let Batch::Single(single) = batch {
            assert_eq!(single.epoch_num, 10);
            assert_eq!(single.timestamp, 4);
        } else {
            panic!("Wrong batch type");
        }

        let err = stream.next_batch(Default::default(), &mock_origins).await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
        assert_eq!(stream.buffer.len(), 0);
        assert!(stream.span.is_none());

        // Add more data into the provider, see if the buffer is re-hydrated.
        stream.prev.batches.push(Ok(Batch::Span(mock_batch)));

        // The next batches should be single batches derived from the span batch.
        let batch = stream.next_batch(Default::default(), &mock_origins).await.unwrap();
        if let Batch::Single(single) = batch {
            assert_eq!(single.epoch_num, 10);
            assert_eq!(single.timestamp, 2);
        } else {
            panic!("Wrong batch type");
        }

        let batch = stream.next_batch(Default::default(), &mock_origins).await.unwrap();
        if let Batch::Single(single) = batch {
            assert_eq!(single.epoch_num, 10);
            assert_eq!(single.timestamp, 4);
        } else {
            panic!("Wrong batch type");
        }

        let err = stream.next_batch(Default::default(), &mock_origins).await.unwrap_err();
        assert_eq!(err, PipelineError::Eof.temp());
        assert_eq!(stream.buffer.len(), 0);
        assert!(stream.span.is_none());
    }

    #[tokio::test]
    async fn test_single_batch_pass_through() {
        let data = vec![Ok(Batch::Single(SingleBatch::default()))];
        let config = Arc::new(RollupConfig { holocene_time: Some(0), ..RollupConfig::default() });
        let prev = MockBatchStreamProvider::new(data);
        let mut stream = BatchStream::new(prev, config.clone(), TestL2ChainProvider::default());

        // The stage should be active.
        assert!(stream.is_active().unwrap());

        // The next batch should be passed through to the [BatchQueue] stage.
        let batch = stream.next_batch(Default::default(), &[]).await.unwrap();
        assert!(matches!(batch, Batch::Single(_)));
        assert_eq!(stream.buffer.len(), 0);
        assert!(stream.span.is_none());
    }
}
