//! This module contains the `BatchQueue` stage implementation.

use crate::stages::channel_reader::ChannelReader;
use crate::traits::{ChainProvider, DataAvailabilityProvider, ResettableStage, SafeBlockFetcher};
use crate::types::{Batch, BatchWithInclusionBlock, SingleBatch};
use crate::types::{BlockInfo, L2BlockRef};
use crate::types::{RollupConfig, SystemConfig};
use crate::types::{StageError, StageResult};
use alloc::boxed::Box;
use alloc::vec::Vec;
use anyhow::anyhow;
use async_trait::async_trait;
use core::fmt::Debug;

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
pub struct BatchQueue<DAP, CP, BF>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
    BF: SafeBlockFetcher + Debug,
{
    /// The rollup config.
    cfg: RollupConfig,
    /// The previous stage of the derivation pipeline.
    prev: ChannelReader<DAP, CP>,
    /// The l1 block ref
    origin: Option<BlockInfo>,

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

impl<DAP, CP, BF> BatchQueue<DAP, CP, BF>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
    BF: SafeBlockFetcher + Debug,
{
    /// Creates a new [BatchQueue] stage.
    pub fn new(cfg: RollupConfig, prev: ChannelReader<DAP, CP>, fetcher: BF) -> Self {
        Self {
            cfg,
            prev,
            origin: None,
            l1_blocks: Vec::new(),
            batches: Vec::new(),
            next_spans: Vec::new(),
            fetcher,
        }
    }

    /// Returns the L1 origin [BlockInfo].
    pub fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }

    /// Pops the next batch from the current queued up span-batch cache.
    /// The parent is used to set the parent hash of the batch.
    /// The parent is verified when the batch is later validated.
    pub fn pop_next_batch(&mut self, parent: L2BlockRef) -> Option<SingleBatch> {
        if self.next_spans.is_empty() {
            return None;
        }
        let mut next = self.next_spans.remove(0);
        next.parent_hash = parent.info.hash;
        Some(next)
    }

    /// Returns the next valid batch upon the given safe head.
    /// Also returns the boolean that indicates if the batch is the last block in the batch.
    pub async fn next_batch(&mut self, parent: L2BlockRef) -> StageResult<SingleBatch> {
        if !self.next_spans.is_empty() {
            // There are cached singular batches derived from the span batch.
            // Check if the next cached batch matches the given parent block.
            if self.next_spans[0].timestamp == parent.info.timestamp + self.cfg.block_time {
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
                    self.l1_blocks.drain(0..=i);
                    // TODO: log that the pipelien has advanced the epoch.
                    // TODO: metrice the internal epoch advancement.
                    break;
                }
            }
            // If the origin of the parent block is not included, we must advance the origin.
        }

        // NOTE: The origin is used to determine if it's behind.
        // It is the future origin that gets saved into the l1 blocks array.
        // We always update the origin of this stage if it's not the same so
        // after the update code runs, this is consistent.
        let origin_behind = self
            .origin
            .map_or(true, |origin| origin.number < parent.l1_origin.number);

        // Advance the origin if needed.
        // The entire pipeline has the same origin.
        // Batches prior to the l1 origin of the l2 safe head are not accepted.
        if self.origin != self.prev.origin().copied() {
            self.origin = self.prev.origin().cloned();
            if !origin_behind {
                self.l1_blocks.push(*self.origin.as_ref().unwrap());
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
            return Err(StageError::Custom(anyhow!("Not Enough Data")));
        }

        // Attempt to derive more batches.
        let batch = match self.derive_next_batch(out_of_data, parent) {
            Ok(b) => b,
            Err(e) => match e {
                StageError::Eof => {
                    if out_of_data {
                        return Err(StageError::Eof);
                    }
                    return Err(StageError::Custom(anyhow!("Not Enough Data")));
                }
                _ => return Err(e),
            },
        };

        // If the next batch is derived from the span batch, it's the last batch of the span.
        // For singular batches, the span batch cache should be empty.
        match batch {
            Batch::Single(sb) => Ok(sb),
            Batch::Span(sb) => {
                let batches = sb.get_singular_batches(&self.l1_blocks, parent); // .ok_or_else(|| anyhow!("failed to get singular batches from span batch"))?;
                self.next_spans = batches;
                let nb = self
                    .pop_next_batch(parent)
                    .ok_or_else(|| anyhow!("failed to pop next batch from span batch"))?;
                Ok(nb)
            }
        }
    }

    /// Derives the next batch.
    pub fn derive_next_batch(&self, _empty: bool, _parent: L2BlockRef) -> StageResult<Batch> {
        unimplemented!()
    }

    /// Adds a batch to the queue.
    pub fn add_batch(&mut self, batch: Batch, parent: L2BlockRef) -> StageResult<()> {
        if self.l1_blocks.is_empty() {
            return Err(anyhow!(
                "cannot add batch with timestamp {}, no origin was prepared",
                batch.timestamp()
            )
            .into());
        }
        let origin = self
            .origin
            .ok_or_else(|| anyhow!("cannot add batch with missing origin"))?;
        let data = BatchWithInclusionBlock {
            inclusion_block: origin,
            batch,
        };
        // If we drop the batch, validation logs the drop reason with WARN level.
        if data
            .check_batch(&self.cfg, &self.l1_blocks, parent, &self.fetcher)
            .is_drop()
        {
            return Ok(());
        }
        self.batches.push(data);
        Ok(())
    }
}

#[async_trait]
impl<DAP, CP, BF> ResettableStage for BatchQueue<DAP, CP, BF>
where
    DAP: DataAvailabilityProvider + Send + Debug,
    CP: ChainProvider + Send + Debug,
    BF: SafeBlockFetcher + Send + Debug,
{
    async fn reset(&mut self, base: BlockInfo, _: SystemConfig) -> StageResult<()> {
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
