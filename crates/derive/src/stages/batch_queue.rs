//! This module contains the `BatchQueue` stage implementation.

use crate::{
    stages::attributes_queue::AttributesProvider,
    traits::{LogLevel, OriginProvider, ResettableStage, SafeBlockFetcher, TelemetryProvider},
    types::{
        Batch, BatchValidity, BatchWithInclusionBlock, BlockInfo, L2BlockInfo, RollupConfig,
        SingleBatch, StageError, StageResult, SystemConfig,
    },
};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::Bytes;
use anyhow::anyhow;
use async_trait::async_trait;
use core::fmt::Debug;

/// Provides batches for the [BatchQueue] stage.
#[async_trait]
pub trait BatchQueueProvider {
    /// Returns the next batch in the [ChannelReader] stage, if the stage is not complete.
    /// This function can only be called once while the stage is in progress, and will return
    /// [`None`] on subsequent calls unless the stage is reset or complete. If the stage is
    /// complete and the batch has been consumed, an [StageError::Eof] error is returned.
    async fn next_batch(&mut self) -> StageResult<Batch>;
}

/// [BatchQueue] is responsible for o rdering unordered batches
/// and gnerating empty batches when the sequence window has passed.
///
/// It receives batches that are tagged with the L1 Inclusion block of the batch.
/// It only considers batches that are inside the sequencing window of a specific L1 Origin.
/// It tries to eagerly pull batches based on the current L2 safe head.
/// Otherwise it filters/creates an entire epoch's worth of batches at once.
///
/// This stage tracks a range of L1 blocks with the assumption that all batches with an L1 inclusion
/// block inside that range have been added to the stage by the time that it attempts to advance a
/// full epoch.
///
/// It is internally responsible for making sure that batches with L1 inclusions block outside it's
/// working range are not considered or pruned.
#[derive(Debug)]
pub struct BatchQueue<P, BF, T>
where
    P: BatchQueueProvider + OriginProvider + Debug,
    BF: SafeBlockFetcher + Debug,
    T: TelemetryProvider + Debug,
{
    /// The rollup config.
    cfg: RollupConfig,
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// The l1 block ref
    origin: Option<BlockInfo>,

    /// Telemetry
    telemetry: T,

    /// A consecutive, time-centric window of L1 Blocks.
    /// Every L1 origin of unsafe L2 Blocks must be included in this list.
    /// If every L2 Block corresponding to a single L1 Block becomes safe,
    /// the block is popped from this list.
    /// If new L2 Block's L1 origin is not included in this list, fetch and
    /// push it to the list.
    l1_blocks: Vec<BlockInfo>,

    /// A set of batches in order from when we've seen them.
    batches: Vec<BatchWithInclusionBlock>,

    /// A set of cached [SingleBatche]s derived from [SpanBatch]s.
    next_spans: Vec<SingleBatch>,

    /// Used to validate the batches.
    fetcher: BF,
}

impl<P, BF, T> BatchQueue<P, BF, T>
where
    P: BatchQueueProvider + OriginProvider + Debug,
    BF: SafeBlockFetcher + Debug,
    T: TelemetryProvider + Debug,
{
    /// Creates a new [BatchQueue] stage.
    pub fn new(cfg: RollupConfig, prev: P, telemetry: T, fetcher: BF) -> Self {
        Self {
            cfg,
            prev,
            origin: None,
            telemetry,
            l1_blocks: Vec::new(),
            batches: Vec::new(),
            next_spans: Vec::new(),
            fetcher,
        }
    }

    /// Pops the next batch from the current queued up span-batch cache.
    /// The parent is used to set the parent hash of the batch.
    /// The parent is verified when the batch is later validated.
    pub fn pop_next_batch(&mut self, parent: L2BlockInfo) -> Option<SingleBatch> {
        if self.next_spans.is_empty() {
            panic!("Invalid state: must have next spans to pop");
        }
        let mut next = self.next_spans.remove(0);
        next.parent_hash = parent.block_info.hash;
        Some(next)
    }

    /// Derives the next batch to apply on top of the current L2 safe head.
    /// Follows the validity rules imposed on consecutive batches.
    /// Based on currently available buffered batch and L1 origin information.
    /// A [StageError::Eof] is returned if no batch can be derived yet.
    pub fn derive_next_batch(&mut self, empty: bool, parent: L2BlockInfo) -> StageResult<Batch> {
        // Cannot derive a batch if no origin was prepared.
        if self.l1_blocks.is_empty() {
            return Err(StageError::MissingOrigin);
        }

        // Get the epoch
        let epoch = self.l1_blocks[0];
        // TODO: log that the next batch is being derived.
        // TODO: metrice the time it takes to derive the next batch.

        // Note: epoch origin can now be one block ahead of the L2 Safe Head
        // This is in the case where we auto generate all batches in an epoch & advance the epoch
        // but don't advance the L2 Safe Head's epoch
        if parent.l1_origin != epoch.id() && parent.l1_origin.number != epoch.number - 1 {
            return Err(StageError::Custom(anyhow!(
                "buffered L1 chain epoch {} in batch queue does not match safe head origin {:?}",
                epoch,
                parent.l1_origin
            )));
        }

        // Find the first-seen batch that matches all validity conditions.
        // We may not have sufficient information to proceed filtering, and then we stop.
        // There may be none: in that case we force-create an empty batch
        let mut next_batch = None;
        let next_timestamp = parent.block_info.timestamp + self.cfg.block_time;

        // Go over all batches, in order of inclusion, and find the first batch we can accept.
        // Filter in-place by only remembering the batches that may be processed in the future, or
        // any undecided ones.
        let mut remaining = Vec::new();
        for i in 0..self.batches.len() {
            let batch = &self.batches[i];
            let validity = batch.check_batch(&self.cfg, &self.l1_blocks, parent, &self.fetcher);
            match validity {
                BatchValidity::Future => {
                    remaining.push(batch.clone());
                }
                BatchValidity::Drop => {
                    // TODO: Log the drop reason with WARN level.
                    // batch.log_context(self.log).warn("Dropping batch", "parent", parent.id(),
                    // "parent_time", parent.info.time);
                    continue;
                }
                BatchValidity::Accept => {
                    next_batch = Some(batch.clone());
                    // Don't keep the current batch in the remaining items since we are processing
                    // it now, but retain every batch we didn't get to yet.
                    remaining.extend_from_slice(&self.batches[i + 1..]);
                    break;
                }
                BatchValidity::Undecided => {
                    remaining.extend_from_slice(&self.batches[i..]);
                    self.batches = remaining;
                    return Err(StageError::Eof);
                }
            }
        }
        self.batches = remaining;

        if let Some(nb) = next_batch {
            // TODO: log that the next batch is found.
            return Ok(nb.batch);
        }

        // If the current epoch is too old compared to the L1 block we are at,
        // i.e. if the sequence window expired, we create empty batches for the current epoch
        let expiry_epoch = epoch.number + self.cfg.seq_window_size;
        let force_empty_batches = (expiry_epoch == parent.l1_origin.number && empty) ||
            expiry_epoch < parent.l1_origin.number;
        let first_of_epoch = epoch.number == parent.l1_origin.number + 1;

        // TODO: Log the empty batch generation.

        // If the sequencer window did not expire,
        // there is still room to receive batches for the current epoch.
        // No need to force-create empty batch(es) towards the next epoch yet.
        if !force_empty_batches {
            return Err(StageError::Eof);
        }

        // The next L1 block is needed to proceed towards the next epoch.
        if self.l1_blocks.len() < 2 {
            return Err(StageError::Eof);
        }

        let next_epoch = self.l1_blocks[1];

        // Fill with empty L2 blocks of the same epoch until we meet the time of the next L1 origin,
        // to preserve that L2 time >= L1 time. If this is the first block of the epoch, always
        // generate a batch to ensure that we at least have one batch per epoch.
        if next_timestamp < next_epoch.timestamp || first_of_epoch {
            // TODO: log next batch generation.
            return Ok(Batch::Single(SingleBatch {
                parent_hash: parent.block_info.hash,
                epoch_num: epoch.number,
                epoch_hash: epoch.hash,
                timestamp: next_timestamp,
                transactions: Vec::new(),
            }));
        }

        // At this point we have auto generated every batch for the current epoch
        // that we can, so we can advance to the next epoch.
        // TODO: log that the epoch is advanced.
        // bq.log.Trace("Advancing internal L1 blocks", "next_timestamp", nextTimestamp,
        // "next_epoch_time", nextEpoch.Time)
        self.l1_blocks.remove(0);
        Err(StageError::Eof)
    }

    /// Adds a batch to the queue.
    pub fn add_batch(&mut self, batch: Batch, parent: L2BlockInfo) -> StageResult<()> {
        if self.l1_blocks.is_empty() {
            // TODO: log that the batch cannot be added without an origin
            panic!("Cannot add batch without an origin");
        }
        let origin = self.origin.ok_or_else(|| anyhow!("cannot add batch with missing origin"))?;
        let data = BatchWithInclusionBlock { inclusion_block: origin, batch };
        // If we drop the batch, validation logs the drop reason with WARN level.
        if data.check_batch(&self.cfg, &self.l1_blocks, parent, &self.fetcher).is_drop() {
            return Ok(());
        }
        self.batches.push(data);
        Ok(())
    }
}

#[async_trait]
impl<P, BF, T> AttributesProvider for BatchQueue<P, BF, T>
where
    P: BatchQueueProvider + OriginProvider + Send + Debug,
    BF: SafeBlockFetcher + Send + Debug,
    T: TelemetryProvider + Send + Debug,
{
    /// Returns the next valid batch upon the given safe head.
    /// Also returns the boolean that indicates if the batch is the last block in the batch.
    async fn next_batch(&mut self, parent: L2BlockInfo) -> StageResult<SingleBatch> {
        if !self.next_spans.is_empty() {
            // There are cached singular batches derived from the span batch.
            // Check if the next cached batch matches the given parent block.
            if self.next_spans[0].timestamp == parent.block_info.timestamp + self.cfg.block_time {
                return self
                    .pop_next_batch(parent)
                    .ok_or(anyhow!("failed to pop next batch from span batch").into());
            }
            // Parent block does not match the next batch.
            // Means the previously returned batch is invalid.
            // Drop cached batches and find another batch.
            self.next_spans.clear();
            // TODO: log that the provided parent block does not match the next batch.
            // TODO: metrice the internal batch drop.
        }

        // If the epoch is advanced, update the l1 blocks.
        // Advancing epoch must be done after the pipeline successfully applies the entire span
        // batch to the chain.
        // Because the span batch can be reverted during processing the batch, then we must
        // preserve existing l1 blocks to verify the epochs of the next candidate batch.
        if !self.l1_blocks.is_empty() && parent.l1_origin.number > self.l1_blocks[0].number {
            for (i, block) in self.l1_blocks.iter().enumerate() {
                if parent.l1_origin.number == block.number {
                    self.l1_blocks.drain(0..i);
                    self.telemetry
                        .write(Bytes::from("Advancing internal L1 blocks"), LogLevel::Info);
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
            self.origin.map_or(true, |origin| origin.number < parent.l1_origin.number);

        // Advance the origin if needed.
        // The entire pipeline has the same origin.
        // Batches prior to the l1 origin of the l2 safe head are not accepted.
        if self.origin != self.prev.origin().copied() {
            self.origin = self.prev.origin().cloned();
            if !origin_behind {
                let origin = self.origin.as_ref().ok_or_else(|| anyhow!("missing origin"))?;
                self.l1_blocks.push(*origin);
            } else {
                // This is to handle the special case of startup.
                // At startup, the batch queue is reset and includes the
                // l1 origin. That is the only time where immediately after
                // reset is called, the origin behind is false.
                self.l1_blocks.clear();
            }
            // TODO: log batch queue origin advancement.
        }

        // Load more data into the batch queue.
        let mut out_of_data = false;
        match self.prev.next_batch().await {
            Ok(b) => {
                if !origin_behind {
                    self.add_batch(b, parent).ok();
                } else {
                    // TODO: metrice when the batch is dropped because the origin is behind.
                }
            }
            Err(StageError::Eof) => out_of_data = true,
            Err(e) => return Err(e),
        }

        // Skip adding the data unless up to date with the origin,
        // but still fully empty the previous stages.
        if origin_behind {
            if out_of_data {
                return Err(StageError::Eof);
            }
            return Err(StageError::NotEnoughData);
        }

        // Attempt to derive more batches.
        let batch = match self.derive_next_batch(out_of_data, parent) {
            Ok(b) => b,
            Err(e) => match e {
                StageError::Eof => {
                    if out_of_data {
                        return Err(StageError::Eof);
                    }
                    return Err(StageError::NotEnoughData);
                }
                _ => return Err(e),
            },
        };

        // If the next batch is derived from the span batch, it's the last batch of the span.
        // For singular batches, the span batch cache should be empty.
        match batch {
            Batch::Single(sb) => Ok(sb),
            Batch::Span(sb) => {
                let batches = sb.get_singular_batches(&self.l1_blocks, parent);
                self.next_spans = batches;
                let nb = self
                    .pop_next_batch(parent)
                    .ok_or_else(|| anyhow!("failed to pop next batch from span batch"))?;
                Ok(nb)
            }
        }
    }

    /// Returns if the previous batch was the last in the span.
    fn is_last_in_span(&self) -> bool {
        self.next_spans.is_empty()
    }
}

impl<P, BF, T> OriginProvider for BatchQueue<P, BF, T>
where
    P: BatchQueueProvider + OriginProvider + Debug,
    BF: SafeBlockFetcher + Debug,
    T: TelemetryProvider + Debug,
{
    fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P, BF, T> ResettableStage for BatchQueue<P, BF, T>
where
    P: BatchQueueProvider + OriginProvider + Send + Debug,
    BF: SafeBlockFetcher + Send + Debug,
    T: TelemetryProvider + Send + Debug + Sync,
{
    async fn reset(&mut self, base: BlockInfo, _: &SystemConfig) -> StageResult<()> {
        // Copy over the Origin from the next stage.
        // It is set in the engine queue (two stages away)
        // such that the L2 Safe Head origin is the progress.
        self.origin = Some(base);
        self.batches.clear();
        // Include the new origin as an origin to build on.
        // This is only for the initialization case.
        // During normal resets we will later throw out this block.
        self.l1_blocks.clear();
        self.l1_blocks.push(base);
        self.next_spans.clear();
        Err(StageError::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        stages::{channel_reader::BatchReader, test_utils::MockBatchQueueProvider},
        traits::test_utils::{MockBlockFetcher, TestTelemetry},
        types::BatchType,
    };
    use alloc::vec;
    use miniz_oxide::deflate::compress_to_vec_zlib;

    fn new_batch_reader() -> BatchReader {
        let raw_data = include_bytes!("../../testdata/raw_batch.hex");
        let mut typed_data = vec![BatchType::Span as u8];
        typed_data.extend_from_slice(raw_data.as_slice());
        let compressed = compress_to_vec_zlib(typed_data.as_slice(), 5);
        BatchReader::from(compressed)
    }

    #[test]
    fn test_derive_next_batch_missing_origin() {
        let telemetry = TestTelemetry::new();
        let data = vec![Ok(Batch::Single(SingleBatch::default()))];
        let cfg = RollupConfig::default();
        let mock = MockBatchQueueProvider::new(data);
        let fetcher = MockBlockFetcher::default();
        let mut bq = BatchQueue::new(cfg, mock, telemetry, fetcher);
        let parent = L2BlockInfo::default();
        let result = bq.derive_next_batch(false, parent).unwrap_err();
        assert_eq!(result, StageError::MissingOrigin);
    }

    #[tokio::test]
    async fn test_next_batch_not_enough_data() {
        let mut reader = new_batch_reader();
        let batch = reader.next_batch().unwrap();
        let mock = MockBatchQueueProvider::new(vec![Ok(batch)]);
        let telemetry = TestTelemetry::new();
        let fetcher = MockBlockFetcher::default();
        let mut bq = BatchQueue::new(RollupConfig::default(), mock, telemetry, fetcher);
        let res = bq.next_batch(L2BlockInfo::default()).await.unwrap_err();
        assert_eq!(res, StageError::NotEnoughData);
        assert!(bq.is_last_in_span());
    }

    // TODO(refcell): The batch reader here loops forever.
    //                Maybe the cursor isn't being used?
    // #[tokio::test]
    // async fn test_next_batch_succeeds() {
    //     let mut reader = new_batch_reader();
    //     let mut batch_vec: Vec<StageResult<Batch>> = vec![];
    //     while let Some(batch) = reader.next_batch() {
    //         batch_vec.push(Ok(batch));
    //     }
    //     let mock = MockBatchQueueProvider::new(batch_vec);
    //     let telemetry = TestTelemetry::new();
    //     let fetcher = MockBlockFetcher::default();
    //     let mut bq = BatchQueue::new(RollupConfig::default(), mock, telemetry, fetcher);
    //     let res = bq.next_batch(L2BlockInfo::default()).await.unwrap();
    //     assert_eq!(res, SingleBatch::default());
    //     assert!(bq.is_last_in_span());
    // }

    #[tokio::test]
    async fn test_batch_queue_empty_bytes() {
        let telemetry = TestTelemetry::new();
        let data = vec![Ok(Batch::Single(SingleBatch::default()))];
        let cfg = RollupConfig::default();
        let mock = MockBatchQueueProvider::new(data);
        let fetcher = MockBlockFetcher::default();
        let mut bq = BatchQueue::new(cfg, mock, telemetry, fetcher);
        let parent = L2BlockInfo::default();
        let result = bq.next_batch(parent).await;
        assert!(result.is_err());
    }
}
