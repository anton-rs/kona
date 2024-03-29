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
